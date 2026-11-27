/* 链接发布 Runtime，用冻结 C ABI 验证提交截止与配额。 */
#if !defined(_WIN32)
#define _POSIX_C_SOURCE 200809L
#endif
#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#if defined(_WIN32)
#include <windows.h>
#else
#include <time.h>
#endif

typedef struct {
    uint32_t tag;
    uint64_t payload;
} XiaoValue;

typedef struct {
    const char *ptr;
    size_t len;
} XiaoBytes;

typedef struct {
    char *ptr;
    size_t capacity;
} XiaoMutBytes;

extern int32_t xiao_runtime_env_run_native(int32_t (*entry)(void), uint64_t capabilities,
                                          int32_t capture_arguments);
extern int32_t xiao_runtime_env_get(const XiaoValue *, XiaoValue *, uint64_t);
extern int32_t xiao_runtime_env_set(const XiaoValue *, const XiaoValue *, uint64_t);
extern int32_t xiao_runtime_env_unset(const XiaoValue *, XiaoValue *, uint64_t);
extern int32_t xiao_runtime_string_new(XiaoBytes, void **);
extern int32_t xiao_runtime_string_copy(void *, XiaoMutBytes, size_t *);
extern int32_t xiao_runtime_value_iter_get(const XiaoValue *, int64_t, XiaoValue *);
extern void xiao_runtime_value_release_strong(XiaoValue *);
extern int32_t xiao_runtime_error_exit_code(void);
extern void xiao_runtime_error_report(void);
extern void xiao_runtime_error_clear(void);

static const uint64_t env_capability = 128;

static XiaoValue text(const char *value) {
    void *handle = NULL;
    assert(xiao_runtime_string_new((XiaoBytes){value, strlen(value)}, &handle) == 0);
    return (XiaoValue){8, (uintptr_t)handle};
}

static void pause_poll(void) {
#if defined(_WIN32)
    Sleep(5);
#else
    nanosleep(&(struct timespec){0, 5000000}, NULL);
#endif
}

static int report(int status, int expected_status, int expected_exit) {
    assert(status == expected_status);
    int code = xiao_runtime_error_exit_code();
    assert(code == expected_exit);
    xiao_runtime_error_report();
    xiao_runtime_error_clear();
    return code;
}

static int32_t probe(void) {
    XiaoValue key = text("key");
    XiaoValue missing = text("N");
    XiaoValue reference = text("ref");
    XiaoValue empty = text("");
    XiaoValue one = text("1");
    XiaoValue two = text("12");
    XiaoValue replacement = text("new!");
    XiaoValue values = {0};
    XiaoValue old = {0};
    char buffer[3];
    size_t len;
    assert(xiao_runtime_env_get(&key, &values, env_capability) == 0);
    assert(xiao_runtime_value_iter_get(&values, 0, &old) == 0 && old.tag == 8);
    assert(xiao_runtime_string_copy((void *)(uintptr_t)old.payload,
                                    (XiaoMutBytes){buffer, sizeof(buffer)}, &len) == 0);
    assert(len == sizeof(buffer) && memcmp(buffer, "old", len) == 0);

    const char *ready_path = getenv("XIAO_B3_CONTROL_READY_FILE");
    if (ready_path != NULL) {
        FILE *ready = fopen(ready_path, "wb");
        assert(ready != NULL && fclose(ready) == 0);
    }
    int32_t status = 0;
    for (int attempt = 0; status == 0 && attempt < 4000; ++attempt) {
        pause_poll();
        status = xiao_runtime_env_get(&missing, &values, env_capability);
    }
    report(status, 8, 2);

    const char *operation = getenv("XIAO_B3_CONTROL_TEST");
    assert(operation != NULL);
    XiaoValue sentinel = {2, 5};
    if (strcmp(operation, "set") == 0) {
        status = xiao_runtime_env_set(&key, &replacement, env_capability);
    } else {
        assert(strcmp(operation, "unset") == 0);
        status = xiao_runtime_env_unset(&key, &sentinel, env_capability);
        assert(sentinel.tag == 2 && sentinel.payload == 5);
    }
    int code = report(status, 8, 2);

    /* 仍满 1024 项。总额距 1 MiB 恰好一字节，所以只允许 +1 进入安全点。 */
    report(xiao_runtime_env_set(&missing, &empty, env_capability), 6, 3);
    report(xiao_runtime_env_set(&reference, &one, env_capability), 8, 2);
    report(xiao_runtime_env_set(&reference, &two, env_capability), 6, 3);
    puts("old");

    XiaoValue *owned[] = {&key, &missing, &reference, &empty, &one, &two, &replacement, &values, &old};
    for (size_t i = 0; i < sizeof(owned) / sizeof(*owned); ++i) {
        xiao_runtime_value_release_strong(owned[i]);
    }
    return code;
}

int main(void) {
    return xiao_runtime_env_run_native(probe, env_capability, 0);
}
