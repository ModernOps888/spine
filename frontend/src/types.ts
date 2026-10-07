export interface VertebraStatus {
  id: string;
  section: 'cervical' | 'thoracic' | 'lumbar' | 'sacral' | 'coccygeal';
  name: string;
  active: boolean;
  score: number;
  detail: string;
}

export interface SpineTelemetrySnapshot {
  vertebrae: VertebraStatus[];
  overall_rigidity: number;
  active_vertebrae_count: number;
  pushbacks_resisted: number;
  apologies_intercepted: number;
  ttft_ms: number;
  tokens_per_sec: number;
  total_tokens: number;
}

export interface ChatMessage {
  role: 'user' | 'assistant' | 'system';
  content: string;
  telemetry?: SpineTelemetrySnapshot;
}

export type RealityLevel = 1 | 2 | 3 | 4;

export interface ModelInfo {
  id: string;
  name: string;
  provider: string;
  description: string;
  is_local: boolean;
}
