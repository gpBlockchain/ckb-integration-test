export interface BenchData {
  id: string;
  date: string;
  n_nodes: number;
  n_inout: number;
  ckb_version: string;
  ckb_version_short: string;
  delay_time_ms: number;
  from_block_number: number;
  to_block_number: number;
  transactions_per_second: number;
  transactions_size_per_second: number;
  average_block_transactions: number;
  average_block_transactions_size: number;
  average_block_time_ms: number;
  total_transactions: number;
  total_transactions_size: number;
  set_send_tps: number;
  client_send_tps: number;
  grafana_link: string;
  report_link: string;
}

export type FilterConfig = 'all' | '3x1' | '3x2' | '3x5' | '3x10';
