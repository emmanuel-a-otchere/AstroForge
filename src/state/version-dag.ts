/**
 * CR-07 B8: version DAG builder (§15).
 *
 * Input: a flat list of `ImageVersion` records. Output: a forest
 * of `DagNode` trees rooted at versions whose `source_version_id`
 * is null or points to a version that isn't in the project (orphan
 * "roots" are still drawn so the user can see them).
 *
 * Edge convention: each non-root node keeps a `parentId` so the
 * tree can be re-rendered incrementally; the rendered tree
 * structure is canonical.
 *
 * Determinism: ties (same parent, multiple children) are broken
 * by `(sequence asc, version_id asc)` so the layout is stable
 * across renders and across users.
 *
 * Hidden nodes are kept in the tree but marked `hidden: true` so
 * the UI can grey them out without dropping them — a hidden
 * version may still be the parent of a visible one.
 *
 * Cycle protection: the schema forbids cycles (inserts must
 * reference an existing version), so we trust the input. If a
 * parent id points to a version that's already in the ancestor
 * chain, we surface it as a `cycleDetected: true` flag rather
 * than looping forever.
 */
import type { ImageVersion } from "../lib/astroforge-api";

export interface DagNode {
  version_id: string;
  label: string;
  sequence: number;
  has_artifact: boolean;
  hidden: boolean;
  children: DagNode[];
  depth: number;
  /** Set when a child claims a parent that's already in this
   *  node's ancestor chain (cycle). The duplicate subtree is
   *  dropped so the renderer doesn't loop. */
  cycleDetected?: boolean;
}

export interface DagBuildResult {
  /** Forest (top-level roots). */
  roots: DagNode[];
  /** Versions not placed in the tree because they were caught
   *  in a cycle. Surfaced so the UI can show "X versions
   *  excluded (cycle)". */
  excluded: ImageVersion[];
  /** Total version count including hidden + cycle-excluded. */
  total: number;
}

export function buildVersionDag(
  versions: ReadonlyArray<ImageVersion>,
): DagBuildResult {
  const byId = new Map<string, ImageVersion>();
  for (const v of versions) byId.set(v.version_id, v);

  // Detect cycles via DFS. If a node is part of a cycle, mark
  // it so the builder can skip its parents.
  const cycleMembers = new Set<string>();
  for (const v of versions) {
    const seen = new Set<string>();
    let cur: ImageVersion | undefined = v;
    let guard = versions.length + 1;
    while (cur && guard-- > 0) {
      if (seen.has(cur.version_id)) {
        for (const id of seen) cycleMembers.add(id);
        break;
      }
      seen.add(cur.version_id);
      const parent_id = cur.source_version_id;
      if (!parent_id) break;
      cur = byId.get(parent_id);
    }
  }

  // Children index: parent_id -> children (sorted).
  const childrenByParent = new Map<string | null, ImageVersion[]>();
  for (const v of versions) {
    if (cycleMembers.has(v.version_id)) continue;
    const key = v.source_version_id ?? null;
    const bucket = childrenByParent.get(key) ?? [];
    bucket.push(v);
    childrenByParent.set(key, bucket);
  }
  for (const bucket of childrenByParent.values()) {
    bucket.sort((a, b) => {
      if (a.sequence !== b.sequence) return a.sequence - b.sequence;
      return a.version_id.localeCompare(b.version_id);
    });
  }

  // Recursive builder. The `chain` is the ancestor set used to
  // short-circuit a recursive reference (defence-in-depth on top
  // of cycleMembers).
  function build(
    v: ImageVersion,
    depth: number,
    chain: Set<string>,
  ): DagNode {
    const cycleDetected = chain.has(v.version_id);
    const node: DagNode = {
      version_id: v.version_id,
      label: v.label,
      sequence: v.sequence,
      has_artifact: Boolean(v.primary_artifact_id),
      hidden: v.hidden,
      children: [],
      depth,
    };
    if (cycleDetected) {
      node.cycleDetected = true;
      return node;
    }
    const nextChain = new Set(chain);
    nextChain.add(v.version_id);
    for (const child of childrenByParent.get(v.version_id) ?? []) {
      node.children.push(build(child, depth + 1, nextChain));
    }
    return node;
  }

  const rootList = childrenByParent.get(null) ?? [];
  const roots = rootList.map((v) => build(v, 0, new Set()));

  // If there are versions whose parent is in the project but
  // was itself excluded from the cycle-set, surface them so the
  // UI can render the orphan as a root rather than dropping it.
  // (Defensive: in practice every non-cycle version with a
  // valid parent should already be reachable from a root.)
  const reachable = new Set<string>();
  function walk(n: DagNode) {
    reachable.add(n.version_id);
    for (const c of n.children) walk(c);
  }
  for (const r of roots) walk(r);

  for (const v of versions) {
    if (
      !cycleMembers.has(v.version_id) &&
      !reachable.has(v.version_id) &&
      v.source_version_id !== null
    ) {
      roots.push(build(v, 0, new Set()));
    }
  }

  return {
    roots,
    excluded: versions.filter((v) => cycleMembers.has(v.version_id)),
    total: versions.length,
  };
}