//! 链接既有B2测试产物的独立探针；不修改产品或生成新的Runtime。
use std::io::{self, BufRead};
use xiao_runtime::{RuntimeValue, lfloat_divide};
use xiao_runtime_abi::*;

unsafe fn text_value(text: &str, decimal: bool) -> XiaoValue {
    let mut handle=std::ptr::null_mut();
    assert_eq!(unsafe{xiao_runtime_string_new(XiaoAbiBytes{ptr:text.as_ptr(),len:text.len()},&mut handle)},0);
    if decimal {unsafe{xiao_runtime_value_lfloat_owned(handle)}} else {unsafe{xiao_runtime_value_str_owned(handle)}}
}

unsafe fn text(value: &XiaoValue) -> String {
    let handle=unsafe{value.payload.handle};
    let mut length=0;assert_eq!(unsafe{xiao_runtime_string_len(handle,&mut length)},0);
    let mut bytes=vec![0;length];let mut written=0;
    assert_eq!(unsafe{xiao_runtime_string_copy(handle,XiaoAbiMutBytes{ptr:bytes.as_mut_ptr(),capacity:bytes.len()},&mut written)},0);
    assert_eq!(length,written);String::from_utf8(bytes).unwrap()
}

fn aliases() {
    unsafe {
        let mut a=text_value("3.50",true);
        let mut b=XiaoValue::none();let mut out=XiaoValue::none();
        let p=XiaoValue::int(5);let mut rounding=text_value("half_even",false);
        assert_eq!(xiao_runtime_lfloat_divide(&a,&a,&p,&rounding,&mut out),0);
        assert_eq!(text(&out),"1");assert_eq!(text(&a),"3.50");
        assert_eq!(xiao_runtime_value_copy(&a,&mut b),0);
        assert_eq!(xiao_runtime_value_copy(&a,&mut out),0);
        assert_eq!(xiao_runtime_lfloat_divide(&a,&b,&p,&rounding,&mut out),0);
        assert_eq!(text(&out),"1");assert_eq!(text(&a),"3.50");assert_eq!(text(&b),"3.50");
        xiao_runtime_value_release_strong(&mut a);
        assert_eq!(xiao_runtime_lfloat_divide(&b,&b,&p,&rounding,&mut out),0);
        assert_eq!(text(&out),"1");assert_eq!(text(&b),"3.50");
        for value in [&mut b,&mut out,&mut rounding]{xiao_runtime_value_release_strong(value);}
    }
    println!("alias_checks=3");
}

fn main() {
    aliases();
    for line in io::stdin().lock().lines() {
        let line=line.unwrap();let args:Vec<_>=line.split('\t').collect();assert_eq!(args.len(),4);
        let a=RuntimeValue::new_lfloat(args[0]).unwrap();let b=RuntimeValue::new_lfloat(args[1]).unwrap();
        let p=RuntimeValue::Int(args[2].parse().unwrap());let mode=RuntimeValue::new_string(args[3]).unwrap();
        match lfloat_divide(&a,&b,&p,&mode){Ok(value)=>println!("{}",value.display_text().unwrap()),Err(error)=>println!("ERR:{}",error.code())}
    }
}
