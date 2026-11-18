/* 22D-FIX.1 C-11：由三平台 host clang 生成真实目标对象，锁定冻结的 C 视图。 */
#include <stddef.h>
#include <stdint.h>

typedef struct {
    uint64_t strong;
    uint64_t weak;
    uint64_t tag;
    uint32_t lock_state;
    uint32_t reserved;
} XiaoSharedHeader;

typedef struct {
    uint64_t runtime_id;
    uint8_t *object_ptr;
} XiaoSharedHandle;

_Static_assert(sizeof(XiaoSharedHeader) == 32, "header size");
_Static_assert(_Alignof(XiaoSharedHeader) == 8, "header alignment");
_Static_assert(offsetof(XiaoSharedHeader, strong) == 0, "strong offset");
_Static_assert(offsetof(XiaoSharedHeader, weak) == 8, "weak offset");
_Static_assert(offsetof(XiaoSharedHeader, tag) == 16, "tag offset");
_Static_assert(offsetof(XiaoSharedHeader, lock_state) == 24, "lock offset");
_Static_assert(offsetof(XiaoSharedHeader, reserved) == 28, "reserved offset");
_Static_assert(sizeof(XiaoSharedHandle) == 16, "handle size");
_Static_assert(_Alignof(XiaoSharedHandle) == 8, "handle alignment");
_Static_assert(offsetof(XiaoSharedHandle, runtime_id) == 0, "runtime ID offset");
_Static_assert(offsetof(XiaoSharedHandle, object_ptr) == 8, "object pointer offset");

int xiao_concurrency_layout_probe(void) { return 0; }
