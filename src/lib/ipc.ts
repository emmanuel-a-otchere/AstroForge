export type PipelineStageId =
  | "compression_cleanup"
  | "ingest"
  | "quality_filter"
  | "debayer"
  | "calibration"
  | "cosmetic_correction"
  | "registration"
  | "plate_solve"
  | "stacking"
  | "narrowband_compose"
  | "background_extraction"
  | "color_calibration"
  | "color_wb"
  | "color_scnr"
  | "narrowband_color_correction"
  | "crop_and_rotate"
  | "stretching"
  | "deconvolution"
  | "noise_reduction"
  | "star_segmentation_and_enhancement"
  | "ai_super_resolution"
  | "final_detail_enhancement"
  | "export";

export type Verbosity = "beginner" | "intermediate" | "expert";
export type DialogMode = "auto" | "confirm" | "manual";
export type TargetType = "deep_sky" | "planetary" | "lunar";
export type BayerPattern = "RGGB" | "BGGR" | "GRBG" | "GBRG";

export interface IngestRequest {
  sourceDir: string;
  targetType: TargetType;
  verbosity: Verbosity;
}

export interface IngestResponse {
  sessionId: string;
  fileCount: number;
  classifications: Record<string, string[]>;
}

export interface StageProgress {
  stageId: PipelineStageId;
  status: "pending" | "running" | "completed" | "failed" | "skipped";
  progress: number;
  metrics?: Record<string, number>;
  error?: string;
}

export interface PipelineStartRequest {
  sessionId: string;
  stages: PipelineStageId[];
  params: Record<string, unknown>;
}

export interface PreviewRequest {
  sessionId: string;
  stageId: PipelineStageId;
}

export interface PreviewResponse {
  stageId: PipelineStageId;
  imageUrl: string;
  metrics: Record<string, number>;
}

export type IpcEvent =
  | { type: "stage_started"; stageId: PipelineStageId }
  | { type: "stage_progress"; stageId: PipelineStageId; progress: number }
  | { type: "stage_completed"; stageId: PipelineStageId; metrics: Record<string, number> }
  | { type: "stage_failed"; stageId: PipelineStageId; error: string }
  | { type: "pipeline_completed" }
  | { type: "pipeline_paused"; stageId: PipelineStageId };
