/** TypeScript 侧协议常量；数值必须与 Rust xiao-driver::protocol 一致。 */

/** 单帧 JSON 负载最大字节数。 */
export const MAX_FRAME_BYTES = 16 * 1024 * 1024;

/** 帧错误的稳定编号。 */
export const FRAME_ERROR_CODE = "X11-PROTOCOL-001";
