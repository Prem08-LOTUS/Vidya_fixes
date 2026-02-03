export interface SystemStatus {
  safe: boolean;
  rh: number;
  temperature: number;
  z_nm: number;
  z_sigma?: number; // [NEW] Uncertainty
  thermal_gradient?: number; // [NEW] Stability
  timestamp: string;
  last_error?: string;
  history?: { z_nm: number; timestamp_ms: number }[];
}
