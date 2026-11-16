/* B1固定源码既有C调用方；只使用历史符号与repr(C)布局。 */
#include <stdint.h>
#include <stddef.h>
#include <stdio.h>
typedef struct { const uint8_t *ptr; size_t len; } Bytes;
typedef struct { uint32_t tag; union { uint64_t raw; int64_t integer; double real; void *handle; } payload; } Value;
_Static_assert(sizeof(Value)==16 && _Alignof(Value)==8, "B1 XiaoValue layout");
_Static_assert(sizeof(Bytes)==16, "B1 bytes layout");
extern uint32_t xiao_runtime_abi_version(void);
extern uint32_t xiao_runtime_abi_minor_version(void);
extern int32_t xiao_runtime_abi_is_compatible(uint32_t,uint32_t);
extern Value xiao_runtime_value_int(int64_t);
extern Value xiao_runtime_value_none(void);
extern int32_t xiao_runtime_value_binary(Bytes,const Value*,const Value*,Value*);
extern void xiao_runtime_value_release(Value*);
int main(void) {
    Value a=xiao_runtime_value_int(19), b=xiao_runtime_value_int(23), out=xiao_runtime_value_none();
    const uint8_t plus[]="add";
    if (xiao_runtime_abi_version()!=1 || !xiao_runtime_abi_is_compatible(1,18)) return 20;
    if (xiao_runtime_value_binary((Bytes){plus,3},&a,&b,&out) || out.tag!=2 || out.payload.integer!=42) return 21;
    xiao_runtime_value_release(&a);xiao_runtime_value_release(&b);xiao_runtime_value_release(&out);
    printf("B5-CALLER result=42 runtime_minor=%u layout=16/8/16\n",xiao_runtime_abi_minor_version());
    return 0;
}
