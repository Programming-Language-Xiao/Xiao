/** CLI 终端颜色策略；颜色只表达状态，不参与错误语义判断。 */

/** 终端颜色开关。 */
export type ColorMode = "auto" | "always" | "never";

/** 终端中允许使用的少量语义角色。 */
export type ColorRole = "success" | "error" | "info" | "accent";

/** 创建颜色器时可注入的终端能力。 */
export interface ColorOptions {
  /** 显式颜色模式。 */
  mode?: ColorMode;
  /** 输出是否连接到 TTY。 */
  isTTY?: boolean;
  /** 是否存在 NO_COLOR 环境变量。 */
  noColor?: boolean;
  /** COLORTERM 的值，用于判断 truecolor。 */
  colorTerm?: string;
  /** TERM 的值；`dumb` 终端不应接收 ANSI。 */
  term?: string;
}

/** 负责把语义角色渲染为 ANSI 或纯文本。 */
export interface Colorizer {
  /** 当前是否会输出 ANSI。 */
  readonly enabled: boolean;
  /** 当前是否使用 24 位颜色。 */
  readonly trueColor: boolean;
  /** 给文本套用一个语义颜色。 */
  color(role: ColorRole, text: string): string;
  /** 去除文本中的 ANSI 控制序列。 */
  strip(text: string): string;
}

const ANSI_PATTERN = /\u001B\[[0-?]*[ -/]*[@-~]/gu;

const RGB_COLORS: Record<ColorRole, readonly [number, number, number]> = {
  success: [76, 175, 80],
  error: [239, 83, 80],
  info: [66, 165, 245],
  accent: [255, 193, 7],
};

const ANSI_COLORS: Record<ColorRole, number> = {
  success: 32,
  error: 31,
  info: 36,
  accent: 33,
};

/** 构造一个遵守 NO_COLOR、TTY 和显式覆盖顺序的颜色器。 */
export function createColorizer(options: ColorOptions = {}): Colorizer {
  const mode = options.mode ?? "auto";
  const isTTY = options.isTTY ?? Boolean(process.stdout.isTTY);
  const noColor = options.noColor ?? process.env.NO_COLOR !== undefined;
  const dumbTerminal = (options.term ?? process.env.TERM ?? "").toLowerCase() === "dumb";
  const enabled = mode === "always" || (mode !== "never" && !noColor && !dumbTerminal && isTTY);
  const trueColor = enabled && /truecolor|24bit/iu.test(options.colorTerm ?? process.env.COLORTERM ?? "");
  return {
    enabled,
    trueColor,
    /** 给文本套用当前语义色。 */
    color(role, text) {
      if (!enabled || text.length === 0) return text;
      if (trueColor) {
        const [red, green, blue] = RGB_COLORS[role];
        return `\u001B[38;2;${red};${green};${blue}m${text}\u001B[39m`;
      }
      return `\u001B[${ANSI_COLORS[role]}m${text}\u001B[39m`;
    },
    /** 移除文本中的 ANSI 控制序列。 */
    strip(text) {
      return text.replace(ANSI_PATTERN, "");
    },
  };
}

/** 移除 ANSI 控制序列，供管道和快照使用。 */
export function stripAnsi(text: string): string {
  return text.replace(ANSI_PATTERN, "");
}
