declare module '@tauri-apps/api/core' {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  export function invoke(cmd: string, args?: Record<string, unknown>): Promise<any>;
}
