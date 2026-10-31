//! 22E 元数据拒绝路径；每个未知/缺失要求经真实 codec 或对象节读取器校验。

use super::*;

/// 生成当前版本、精确 ABI 区间的有效元数据。
fn current() -> NativeMetadata {
    NativeMetadata {
        version: 1,
        runtime_abi_min: ABI_ENCODED_VERSION,
        runtime_abi_max: ABI_ENCODED_VERSION,
        required_features: xiao_runtime_abi::REQUIRED_FEATURE_THREADS,
        generator_version: CODEGEN_VERSION,
        target: TargetDescription::host().triple,
        pointer_width: 64,
        stdlib: current_standard_library_identity(),
    }
}

#[test]
/// 编解码保持身份，截断、坏 magic、版本及长度都结构化拒绝，不越界读取。
fn native_metadata_codec_is_bounded_and_exact() {
    let metadata = current();
    let bytes = metadata.encode().unwrap();
    assert_eq!(NativeMetadata::decode(&bytes).unwrap(), metadata);
    for length in 0..bytes.len() {
        assert_eq!(
            NativeMetadata::decode(&bytes[..length]).unwrap_err().code,
            "X17-XAR-007"
        );
    }
    for offset in [0, 8, 10, 12, 46] {
        let mut corrupt = bytes.clone();
        corrupt[offset] ^= 0x80;
        assert_eq!(
            NativeMetadata::decode(&corrupt).unwrap_err().code,
            "X17-XAR-007"
        );
    }
    let mut trailing = bytes;
    trailing.push(0);
    assert!(NativeMetadata::decode(&trailing).is_err());
}

#[test]
/// 旧 Runtime、未知位、缺要求、目标、生成器和标准库漂移分别经过正式校验入口。
fn native_metadata_requirements_reject_incompatible_inputs() {
    let target = TargetDescription::host();
    let metadata = current();
    let support = NativeRuntimeSupport::default();
    metadata
        .validate(&target, &support, metadata.required_features)
        .unwrap();
    for abi in [
        xiao_runtime_abi::encoded_abi_version(1, 16),
        xiao_runtime_abi::encoded_abi_version(1, 18),
    ] {
        assert_eq!(
            metadata
                .validate(
                    &target,
                    &NativeRuntimeSupport {
                        runtime_abi: abi,
                        ..support.clone()
                    },
                    0
                )
                .unwrap_err()
                .code,
            "X17-XAR-007"
        );
    }
    let mut value = metadata.clone();
    value.required_features |= 1 << 63;
    assert_eq!(
        value.validate(&target, &support, 0).unwrap_err().code,
        "X17-XAR-011"
    );
    value.required_features = 1;
    assert_eq!(
        value.validate(&target, &support, 0).unwrap_err().code,
        "X17-XAR-011"
    );
    assert_eq!(
        metadata
            .validate(
                &target,
                &NativeRuntimeSupport {
                    features: 0,
                    ..support.clone()
                },
                0
            )
            .unwrap_err()
            .code,
        "X17-XAR-007"
    );
    assert_eq!(
        metadata
            .validate(
                &target,
                &support,
                xiao_runtime_abi::REQUIRED_FEATURE_TASK_POOL
            )
            .unwrap_err()
            .code,
        "X17-XAR-007"
    );
    for update in [0, 1, 2, 3] {
        let mut value = metadata.clone();
        match update {
            0 => value.target = "other-target".to_owned(),
            1 => value.generator_version += 1,
            2 => value.stdlib.stdlib_interface_version += 1,
            _ => value.stdlib.target = "native".to_owned(),
        }
        assert_eq!(
            value.validate(&target, &support, 0).unwrap_err().code,
            "X17-XAR-007"
        );
    }
    let mut value = metadata;
    value.pointer_width = 32;
    assert_eq!(
        value.validate(&target, &support, 0).unwrap_err().code,
        "X20-TARGET-001"
    );
}

#[test]
/// magic 不能出现在普通字节中就被认作节；所有容器截断均失败关闭。
fn native_metadata_does_not_scan_for_magic() {
    for target in [
        TargetDescription::windows_x86_64(),
        TargetDescription::linux_x86_64(),
        TargetDescription::macos_aarch64(),
    ] {
        for bytes in [b"XIAOMETA".as_slice(), &[0; 64]] {
            assert_eq!(
                read_native_metadata_bytes(bytes, &target).unwrap_err().code,
                "X17-XAR-007"
            );
        }
    }
}

#[test]
/// 在任意宿主为三个目标发射相同记录格式，不把宿主对象格式带入跨目标身份校验。
fn native_metadata_emission_uses_the_declared_target() {
    for target in [
        TargetDescription::windows_x86_64(),
        TargetDescription::linux_x86_64(),
        TargetDescription::macos_aarch64(),
    ] {
        let mut metadata = current();
        metadata.target.clone_from(&target.triple);
        let bytes = metadata.encode().unwrap();
        assert_eq!(NativeMetadata::decode(&bytes).unwrap(), metadata);
        let text = emit(&metadata, target.object_format).unwrap();
        assert!(text.contains("@llvm.used"));
        let expected = match target.object_format {
            ObjectFormat::Coff => ".xiaomta",
            ObjectFormat::Elf => ".note.xiao",
            ObjectFormat::MachO => "__XIAO,__meta",
        };
        assert!(text.contains(expected));
    }
}
