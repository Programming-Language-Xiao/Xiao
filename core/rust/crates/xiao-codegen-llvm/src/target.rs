//! 规范化目标描述；不读取 CLI 或宿主工具链配置。

use crate::error::{CodegenError, Result};

/// 字节序。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Endian {
    /// 小端目标。
    Little,
    /// 大端目标。
    Big,
}

/// 目标文件格式。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ObjectFormat {
    /// Windows PE/COFF。
    Coff,
    /// Linux/Unix ELF。
    Elf,
    /// macOS Mach-O。
    MachO,
}

/// LLVM 后端使用的规范化目标描述。
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct TargetDescription {
    /// LLVM target triple。
    pub triple: String,
    /// 目标指针宽度（32 或 64）。
    pub pointer_width: u16,
    /// 目标字节序。
    pub endian: Endian,
    /// 目标文件格式。
    pub object_format: ObjectFormat,
}

impl TargetDescription {
    /// 创建目标描述并检查固定宽度字段。
    pub fn new(
        triple: impl Into<String>,
        pointer_width: u16,
        endian: Endian,
        object_format: ObjectFormat,
    ) -> Result<Self> {
        let triple = triple.into();
        if triple.trim().is_empty() {
            return Err(CodegenError::InvalidTarget {
                message: "target triple 不能为空".to_owned(),
            });
        }
        if !matches!(pointer_width, 32 | 64) {
            return Err(CodegenError::InvalidTarget {
                message: format!("指针宽度必须是 32 或 64（收到 {pointer_width}）"),
            });
        }
        let target = Self {
            triple,
            pointer_width,
            endian,
            object_format,
        };
        target.validate()?;
        Ok(target)
    }

    /// 验证固定宽度、已知平台三元组与对象格式之间的一致性。
    pub fn validate(&self) -> Result<()> {
        if self.triple.trim().is_empty() {
            return Err(CodegenError::InvalidTarget {
                message: "target triple 不能为空".to_owned(),
            });
        }
        if !matches!(self.pointer_width, 32 | 64) {
            return Err(CodegenError::InvalidTarget {
                message: format!("指针宽度必须是 32 或 64（收到 {}）", self.pointer_width),
            });
        }
        let architecture = self.triple.split('-').next().unwrap_or_default();
        let expected_width = match architecture {
            "x86_64" | "aarch64" | "arm64" => Some(64),
            "i386" | "i686" | "arm" | "armv7" => Some(32),
            _ => {
                return Err(CodegenError::InvalidTarget {
                    message: format!("目标架构 {architecture} 不受支持"),
                });
            }
        };
        if let Some(expected_width) = expected_width
            && self.pointer_width != expected_width
        {
            return Err(CodegenError::InvalidTarget {
                message: format!(
                    "目标架构 {} 要求 {} 位指针（收到 {}）",
                    architecture, expected_width, self.pointer_width
                ),
            });
        }
        let expected_format = if self.triple.contains("windows") {
            Some(ObjectFormat::Coff)
        } else if self.triple.contains("apple") || self.triple.contains("darwin") {
            Some(ObjectFormat::MachO)
        } else if self.triple.contains("linux") {
            Some(ObjectFormat::Elf)
        } else {
            return Err(CodegenError::InvalidTarget {
                message: format!("目标平台 {} 不受支持", self.triple),
            });
        };
        if let Some(expected_format) = expected_format
            && self.object_format != expected_format
        {
            return Err(CodegenError::InvalidTarget {
                message: format!(
                    "目标三元组 {} 要求 {:?} 对象格式（收到 {:?}）",
                    self.triple, expected_format, self.object_format
                ),
            });
        }
        Ok(())
    }

    /// 创建 Windows x86_64 MSVC 目标描述。
    #[must_use]
    pub fn windows_x86_64() -> Self {
        Self {
            triple: "x86_64-pc-windows-msvc".to_owned(),
            pointer_width: 64,
            endian: Endian::Little,
            object_format: ObjectFormat::Coff,
        }
    }

    /// 创建 Windows AArch64 MSVC 目标描述。
    #[must_use]
    pub fn windows_aarch64() -> Self {
        Self {
            triple: "aarch64-pc-windows-msvc".to_owned(),
            pointer_width: 64,
            endian: Endian::Little,
            object_format: ObjectFormat::Coff,
        }
    }

    /// 创建 Linux x86_64 GNU 目标描述。
    #[must_use]
    pub fn linux_x86_64() -> Self {
        Self {
            triple: "x86_64-unknown-linux-gnu".to_owned(),
            pointer_width: 64,
            endian: Endian::Little,
            object_format: ObjectFormat::Elf,
        }
    }

    /// 创建 Linux AArch64 GNU 目标描述。
    #[must_use]
    pub fn linux_aarch64() -> Self {
        Self {
            triple: "aarch64-unknown-linux-gnu".to_owned(),
            pointer_width: 64,
            endian: Endian::Little,
            object_format: ObjectFormat::Elf,
        }
    }

    /// 创建 macOS x86_64 目标描述。
    #[must_use]
    pub fn macos_x86_64() -> Self {
        Self {
            triple: "x86_64-apple-darwin".to_owned(),
            pointer_width: 64,
            endian: Endian::Little,
            object_format: ObjectFormat::MachO,
        }
    }

    /// 创建 macOS AArch64 目标描述。
    #[must_use]
    pub fn macos_aarch64() -> Self {
        Self {
            triple: "aarch64-apple-darwin".to_owned(),
            pointer_width: 64,
            endian: Endian::Little,
            object_format: ObjectFormat::MachO,
        }
    }

    /// 返回当前编译主机的规范化目标；只读取编译期平台，不发现工具链。
    #[must_use]
    pub fn host() -> Self {
        #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
        {
            Self::windows_x86_64()
        }
        #[cfg(all(target_os = "windows", target_arch = "aarch64"))]
        {
            Self::windows_aarch64()
        }
        #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
        {
            Self::macos_x86_64()
        }
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        {
            Self::macos_aarch64()
        }
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        {
            Self::linux_x86_64()
        }
        #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
        {
            Self::linux_aarch64()
        }
        #[cfg(not(any(
            all(target_os = "windows", target_arch = "x86_64"),
            all(target_os = "windows", target_arch = "aarch64"),
            all(target_os = "macos", target_arch = "x86_64"),
            all(target_os = "macos", target_arch = "aarch64"),
            all(target_os = "linux", target_arch = "x86_64"),
            all(target_os = "linux", target_arch = "aarch64"),
        )))]
        {
            Self {
                triple: "unknown-unknown-unknown".to_owned(),
                pointer_width: 64,
                endian: Endian::Little,
                object_format: ObjectFormat::Elf,
            }
        }
    }

    /// 返回用于构建指纹的稳定目标字段串。
    #[must_use]
    pub fn fingerprint_fields(&self) -> String {
        format!(
            "triple={};pointer_width={};endian={:?};format={:?}",
            self.triple, self.pointer_width, self.endian, self.object_format
        )
    }
}

impl ObjectFormat {
    /// 返回协议和产物诊断使用的稳定对象格式名称。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Coff => "coff",
            Self::Elf => "elf",
            Self::MachO => "macho",
        }
    }
}

#[cfg(test)]
/// 目标描述的架构回归测试。
mod tests {
    use super::{Endian, ObjectFormat, TargetDescription};

    #[test]
    /// 确认主机目标与编译期架构、指针宽度和字节序一致。
    fn host_target_architecture_matches_compile_time_target() {
        let target = TargetDescription::host();
        let expected_architecture = if cfg!(target_arch = "x86_64") {
            "x86_64"
        } else if cfg!(target_arch = "aarch64") {
            "aarch64"
        } else {
            panic!("当前测试只覆盖 x86_64 和 aarch64")
        };
        assert_eq!(target.triple.split('-').next(), Some(expected_architecture));
        assert_eq!(target.pointer_width, usize::BITS as u16);
        assert_eq!(target.endian, Endian::Little);

        if cfg!(target_os = "windows") {
            assert_eq!(target.object_format, ObjectFormat::Coff);
        } else if cfg!(target_os = "macos") {
            assert_eq!(target.object_format, ObjectFormat::MachO);
        } else if cfg!(target_os = "linux") {
            assert_eq!(target.object_format, ObjectFormat::Elf);
        } else {
            panic!("当前测试只覆盖 Windows、macOS 和 Linux")
        }
    }

    #[test]
    /// 三种受控目标都必须保留固定宽度字段和对象格式，不依赖宿主工具链。
    fn controlled_targets_validate_equally() {
        let targets = [
            TargetDescription::windows_x86_64(),
            TargetDescription::linux_x86_64(),
            TargetDescription::macos_x86_64(),
        ];
        for target in targets {
            target.validate().expect("受控目标应通过固定宽度校验");
            assert_eq!(target.pointer_width, 64);
            assert_eq!(target.endian, Endian::Little);
        }
    }

    #[test]
    /// 已知平台的窄化、空三元组和对象格式错配必须被结构化拒绝。
    fn invalid_fixed_width_and_format_are_rejected() {
        let narrow = TargetDescription::new(
            "x86_64-unknown-linux-gnu",
            32,
            Endian::Little,
            ObjectFormat::Elf,
        )
        .expect_err("x86_64 不能声明为 32 位");
        assert!(narrow.to_string().contains("要求 64 位"));

        let mismatch = TargetDescription::new(
            "aarch64-apple-darwin",
            64,
            Endian::Little,
            ObjectFormat::Elf,
        )
        .expect_err("macOS 不能声明为 ELF");
        assert!(mismatch.to_string().contains("MachO"));

        let empty = TargetDescription::new("  ", 64, Endian::Little, ObjectFormat::Elf)
            .expect_err("空三元组必须拒绝");
        assert!(empty.to_string().contains("不能为空"));

        let unknown_arch = TargetDescription::new(
            "riscv64-unknown-linux-gnu",
            64,
            Endian::Little,
            ObjectFormat::Elf,
        )
        .expect_err("未支持的架构必须在工具链前拒绝");
        assert!(unknown_arch.to_string().contains("架构 riscv64 不受支持"));

        let unknown_platform = TargetDescription::new(
            "x86_64-unknown-freebsd",
            64,
            Endian::Little,
            ObjectFormat::Elf,
        )
        .expect_err("未支持的平台必须在工具链前拒绝");
        assert!(unknown_platform.to_string().contains("目标平台"));
    }
}
