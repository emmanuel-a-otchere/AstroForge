#!/usr/bin/env python3
"""Generate the CR-06 P5.1 builtin classical-kernel ONNX models.

The AI Enhancement Studio dispatcher (astroforge-ai::operations) needs a
real ONNX inference path on every machine, including CPU-only CI runners
with no downloaded catalog models. The builtin models are small,
self-authored classical image-processing kernels expressed as ONNX
graphs: no training data, no external weights, no licensing surface
(they are authored by this script and carry the repo's license).

Every graph takes a single-channel image tensor `[1, 1, H, W]` with
dynamic spatial dims; the engine loops over image channels and tiles.
Per-operation strength / threshold travels as a scalar graph input so
one graph serves the whole parameter range.

Outputs land in `crates/astroforge-ai/models/builtin/` and the script
prints the sha256 + size of each artifact for the Rust-side manifest in
`crates/astroforge-ai/src/inference.rs`.

Requires: `pip install onnx` (numpy is a transitive dep).
"""

from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path

import numpy as np
import onnx
from onnx import helper, TensorProto

# ONNX Runtime 1.28 (ort 2.0.0-rc.13) supports IR <= 11 and opset <= 23.
# Pin conservatively so the fixtures load on every ORT 1.x that `ort`
# can fetch.
OPSET = 17
IR_VERSION = 9

OUT_DIR = (
    Path(__file__).resolve().parent.parent
    / "crates"
    / "astroforge-ai"
    / "models"
    / "builtin"
)


def _dynamic_image(name: str) -> onnx.ValueInfoProto:
    return helper.make_tensor_value_info(
        name, TensorProto.FLOAT, [1, 1, "H", "W"]
    )


def _scalar(name: str) -> onnx.ValueInfoProto:
    return helper.make_tensor_value_info(name, TensorProto.FLOAT, [1])


def _gaussian3x3() -> np.ndarray:
    """Normalized 3x3 Gaussian kernel (sigma ~0.85), shape [1,1,3,3]."""
    k = np.array([[1.0, 2.0, 1.0], [2.0, 4.0, 2.0], [1.0, 2.0, 1.0]], dtype=np.float32)
    k /= k.sum()
    return k.reshape(1, 1, 3, 3)


def _save(model: onnx.ModelProto, name: str) -> dict:
    model.ir_version = IR_VERSION
    onnx.checker.check_model(model)
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    path = OUT_DIR / f"{name}.onnx"
    data = model.SerializeToString()
    path.write_bytes(data)
    return {
        "name": name,
        "file": str(path.relative_to(Path(__file__).resolve().parent.parent)),
        "sha256": hashlib.sha256(data).hexdigest(),
        "size_bytes": len(data),
    }


def make_blur_blend() -> onnx.ModelProto:
    """out = x * (1 - strength) + gauss3x3(x) * strength."""
    nodes = [
        helper.make_node("Conv", ["image", "kernel"], ["blur"], pads=[1, 1, 1, 1]),
        helper.make_node("Sub", ["one", "strength"], ["inv_strength"]),
        helper.make_node("Mul", ["image", "inv_strength"], ["keep"]),
        helper.make_node("Mul", ["blur", "strength"], ["blended"]),
        helper.make_node("Add", ["keep", "blended"], ["output"]),
    ]
    initializers = [
        helper.make_tensor("kernel", TensorProto.FLOAT, [1, 1, 3, 3], _gaussian3x3().flatten()),
        helper.make_tensor("one", TensorProto.FLOAT, [1], [1.0]),
    ]
    graph = helper.make_graph(
        nodes,
        "builtin-blur-blend",
        [_dynamic_image("image"), _scalar("strength")],
        [_dynamic_image("output")],
        initializer=initializers,
    )
    return helper.make_model(
        graph,
        opset_imports=[helper.make_opsetid("", OPSET)],
        producer_name="astroforge-builtin",
        doc_string="Strength-blended 3x3 Gaussian smoothing. Covers denoise_luminance, denoise_chrominance, star_reduce, background_cleanup.",
    )


def make_sharpen_blend() -> onnx.ModelProto:
    """out = x + strength * (x - gauss3x3(x))  (unsharp mask)."""
    nodes = [
        helper.make_node("Conv", ["image", "kernel"], ["blur"], pads=[1, 1, 1, 1]),
        helper.make_node("Sub", ["image", "blur"], ["detail"]),
        helper.make_node("Mul", ["detail", "strength"], ["scaled_detail"]),
        helper.make_node("Add", ["image", "scaled_detail"], ["output"]),
    ]
    initializers = [
        helper.make_tensor("kernel", TensorProto.FLOAT, [1, 1, 3, 3], _gaussian3x3().flatten()),
    ]
    graph = helper.make_graph(
        nodes,
        "builtin-sharpen-blend",
        [_dynamic_image("image"), _scalar("strength")],
        [_dynamic_image("output")],
        initializer=initializers,
    )
    return helper.make_model(
        graph,
        opset_imports=[helper.make_opsetid("", OPSET)],
        producer_name="astroforge-builtin",
        doc_string="Strength-scaled unsharp mask. Covers detail_enhance, star_refine, deconv (approximation).",
    )


def make_upscale_2x() -> onnx.ModelProto:
    """out = resize(x, scales=[1,1,2,2], mode=linear)."""
    nodes = [
        helper.make_node(
            "Resize",
            ["image", "roi", "scales"],
            ["output"],
            mode="linear",
            coordinate_transformation_mode="align_corners",
        ),
    ]
    initializers = [
        helper.make_tensor("roi", TensorProto.FLOAT, [0], []),
        helper.make_tensor("scales", TensorProto.FLOAT, [4], [1.0, 1.0, 2.0, 2.0]),
    ]
    graph = helper.make_graph(
        nodes,
        "builtin-upscale-2x",
        [_dynamic_image("image")],
        [_dynamic_image("output")],
        initializer=initializers,
    )
    return helper.make_model(
        graph,
        opset_imports=[helper.make_opsetid("", OPSET)],
        producer_name="astroforge-builtin",
        doc_string="Fixed 2x linear upscale. Covers super_resolution (classical stand-in for swinir-sr-astro-2x).",
    )


def make_hotpixel() -> onnx.ModelProto:
    """out = where(x > mean3x3(x) * threshold, mean3x3(x), x).

    Pixels brighter than `threshold` times their 3x3 neighbourhood mean
    are replaced by that mean. Everything else passes through.
    """
    nodes = [
        helper.make_node("Conv", ["image", "kernel"], ["mean"], pads=[1, 1, 1, 1]),
        helper.make_node("Mul", ["mean", "threshold"], ["limit"]),
        helper.make_node("Greater", ["image", "limit"], ["is_hot"]),
        helper.make_node("Where", ["is_hot", "mean", "image"], ["output"]),
    ]
    box = np.ones((1, 1, 3, 3), dtype=np.float32) / 9.0
    initializers = [
        helper.make_tensor("kernel", TensorProto.FLOAT, [1, 1, 3, 3], box.flatten()),
    ]
    graph = helper.make_graph(
        nodes,
        "builtin-hotpixel",
        [_dynamic_image("image"), _scalar("threshold")],
        [_dynamic_image("output")],
        initializer=initializers,
    )
    return helper.make_model(
        graph,
        opset_imports=[helper.make_opsetid("", OPSET)],
        producer_name="astroforge-builtin",
        doc_string="Thresholded outlier replacement (3x3 mean). Covers hot_pixel_clean.",
    )


def make_masked_fill() -> onnx.ModelProto:
    """out = mask * gauss3x3(x) + (1 - mask) * x.

    The masked region is filled with the local Gaussian estimate; the
    unmasked region passes through untouched. Classical stand-in for
    the LaMa-style inpainting catalog models.
    """
    nodes = [
        helper.make_node("Conv", ["image", "kernel"], ["fill"], pads=[1, 1, 1, 1]),
        helper.make_node("Mul", ["mask", "fill"], ["masked_fill"]),
        helper.make_node("Sub", ["one", "mask"], ["inv_mask"]),
        helper.make_node("Mul", ["inv_mask", "image"], ["kept"]),
        helper.make_node("Add", ["masked_fill", "kept"], ["output"]),
    ]
    initializers = [
        helper.make_tensor("kernel", TensorProto.FLOAT, [1, 1, 3, 3], _gaussian3x3().flatten()),
        helper.make_tensor("one", TensorProto.FLOAT, [1], [1.0]),
    ]
    graph = helper.make_graph(
        nodes,
        "builtin-masked-fill",
        [_dynamic_image("image"), _dynamic_image("mask")],
        [_dynamic_image("output")],
        initializer=initializers,
    )
    return helper.make_model(
        graph,
        opset_imports=[helper.make_opsetid("", OPSET)],
        producer_name="astroforge-builtin",
        doc_string="Mask-weighted Gaussian fill. Covers inpaint and trail_clean until the catalog inpainting models ship.",
    )


def main() -> int:
    builders = {
        "builtin-blur-blend": make_blur_blend,
        "builtin-sharpen-blend": make_sharpen_blend,
        "builtin-upscale-2x": make_upscale_2x,
        "builtin-hotpixel": make_hotpixel,
        "builtin-masked-fill": make_masked_fill,
    }
    manifest = []
    for name, build in builders.items():
        manifest.append(_save(build(), name))
    print(json.dumps(manifest, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
