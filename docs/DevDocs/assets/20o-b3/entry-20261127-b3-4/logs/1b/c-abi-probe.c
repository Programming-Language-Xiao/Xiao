#include <assert.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

typedef struct {
    const uint8_t *ptr;
    size_t len;
} XiaoAbiBytes;

typedef struct {
    uint32_t tag;
    union {
        uint64_t raw;
        uint8_t bool_value;
        void *handle;
    } payload;
} XiaoValue;

_Static_assert(sizeof(XiaoValue) == 16, "XiaoValue size");
_Static_assert(offsetof(XiaoValue, payload) == 8, "XiaoValue payload offset");

extern int32_t xiao_runtime_string_new(XiaoAbiBytes, void **);
extern int32_t xiao_runtime_array_len(void *, size_t *);
extern void xiao_runtime_value_release(XiaoValue *);
extern void xiao_runtime_error_clear(void);
extern int32_t xiao_runtime_env_get(const XiaoValue *, XiaoValue *, uint64_t);
extern int32_t xiao_runtime_env_set(const XiaoValue *, const XiaoValue *, uint64_t);
extern int32_t xiao_runtime_env_unset(const XiaoValue *, XiaoValue *, uint64_t);
extern int32_t xiao_runtime_env_run_injected(int32_t (*)(void), const uint8_t *,
                                            size_t, uint64_t);

static const uint64_t environment = UINT64_C(1) << 7;

static XiaoValue string(const char *text) {
    XiaoValue value = { .tag = 8 };
    XiaoAbiBytes bytes = { (const uint8_t *)text, strlen(text) };
    assert(xiao_runtime_string_new(bytes, &value.payload.handle) == 0);
    return value;
}

static int32_t verify_slots(void) {
    XiaoValue name = string("test_name");
    XiaoValue empty = string("");
    XiaoValue out = string("old");
    XiaoValue invalid_tag = { .tag = 255 };
    XiaoValue bad_handle = { .tag = 8 };
    void *old = out.payload.handle;

    assert(xiao_runtime_env_get(NULL, &out, environment) == 1);
    assert(xiao_runtime_env_get(&name, &name, environment) == 2);
    assert(xiao_runtime_env_get(&invalid_tag, &out, environment) == 2);
    assert(xiao_runtime_env_get(&bad_handle, &out, environment) == 3);
    assert(xiao_runtime_env_get(&name, &out, 0) == 6);
    assert(out.tag == 8 && out.payload.handle == old);
    xiao_runtime_error_clear();

    size_t length = 2;
    assert(xiao_runtime_env_get(&name, &out, environment) == 0);
    assert(out.tag == 11 && xiao_runtime_array_len(out.payload.handle, &length) == 0);
    assert(length == 0);
    assert(xiao_runtime_env_set(&name, &empty, environment) == 0);
    assert(xiao_runtime_env_get(&name, &out, environment) == 0);
    XiaoValue retained = out;
    out = (XiaoValue){0};
    assert(xiao_runtime_env_unset(&name, &out, environment) == 0);
    assert(out.tag == 1 && out.payload.bool_value == 1);
    assert(xiao_runtime_env_unset(&name, &out, environment) == 0);
    assert(out.tag == 1 && out.payload.bool_value == 0);
    assert(xiao_runtime_array_len(retained.payload.handle, &length) == 0);
    assert(length == 1);

    xiao_runtime_value_release(&retained);
    xiao_runtime_value_release(&out);
    xiao_runtime_value_release(&empty);
    xiao_runtime_value_release(&name);
    return 0;
}

int main(void) {
    assert(xiao_runtime_env_run_injected(verify_slots, NULL, 0, environment) == 0);
    XiaoValue name = string("test_name");
    XiaoValue out = { .tag = 2, .payload.raw = 17 };
    assert(xiao_runtime_env_get(&name, &out, environment) == 8);
    assert(out.tag == 2 && out.payload.raw == 17);
    xiao_runtime_error_clear();
    xiao_runtime_value_release(&name);
    puts("C ABI: 0/1/2/3/6/8, borrowed inputs, preserved and replaced output, retained array");
    return 0;
}
