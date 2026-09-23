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
        Ok(Self {
            triple,
            pointer_width,
            endian,
            object_format,
        })
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

#[cfg(test)]
mod tests {
    use super::{Endian, ObjectFormat, TargetDescription};

    #[test]
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
}
