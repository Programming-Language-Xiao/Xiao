//! 不可变字符串索引的 Unicode 语义、共享生命周期和实际拥有内存回归。

use super::*;

#[test]
/// 与独立标准字符迭代逐项比较全部 Unicode 边界，保留 NUL/BOM/组合字符且越界不读内存。
fn cached_indices_preserve_every_scalar_and_out_of_bounds() {
    let mut text = String::from("A\0雪😀e\u{301}\u{feff}");
    for point in [
        0, 0x7f, 0x80, 0x7ff, 0x800, 0xd7ff, 0xe000, 0xffff, 0x10000, 0x10ffff,
    ] {
        text.push(char::from_u32(point).unwrap());
    }
    let expected: Vec<_> = text.chars().collect();
    let handle = StringHandle::new(text.clone()).unwrap();
    assert_eq!(handle.len(), expected.len());
    for (index, &character) in expected.iter().enumerate() {
        assert_eq!(handle.character_at(index).unwrap(), Some(character));
    }
    assert_eq!(handle.character_at(expected.len()).unwrap(), None);
    assert_eq!(handle.character_at(usize::MAX).unwrap(), None);
    assert_eq!(handle.to_string().unwrap(), text);
}

#[test]
/// ASCII 与空串无需索引数组；远端位置也不需要重新扫描输入。
fn ascii_and_empty_strings_use_direct_byte_positions() {
    for text in [String::new(), "x".repeat(65_536)] {
        let handle = StringHandle::new(text.clone()).unwrap();
        assert_eq!(handle.len(), text.len());
        assert_eq!(handle.character_at(text.len()).unwrap(), None);
        handle
            .inner
            .with_payload(RuntimeTypeTag::String, |object: &StringObject| {
                assert!(object.scalar_offsets.is_none());
                assert_eq!(object.owned_bytes(), object.value.capacity());
            })
            .unwrap();
        if !text.is_empty() {
            assert_eq!(handle.character_at(65_535).unwrap(), Some('x'));
        }
    }
}

#[test]
/// 克隆只共享同一不可变载荷，销毁原绑定后索引仍活；索引缓冲计入 owned_bytes。
fn shared_unicode_indices_stay_alive_and_are_accounted() {
    let handle = StringHandle::new("雪😀".repeat(1024)).unwrap();
    let alias = handle.clone();
    assert_eq!(handle.strong_count(), 2);
    handle
        .inner
        .with_payload(RuntimeTypeTag::String, |object: &StringObject| {
            let offsets = object.scalar_offsets.as_ref().unwrap();
            assert_eq!(offsets.len(), 2048);
            assert_eq!(
                object.owned_bytes(),
                object.value.capacity() + offsets.capacity() * std::mem::size_of::<usize>()
            );
        })
        .unwrap();
    drop(handle);
    assert_eq!(alias.len(), 2048);
    assert_eq!(alias.character_at(2047).unwrap(), Some('😀'));
}
