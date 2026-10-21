use std::env;
use xiao_concurrency_prototype::{
    candidate_a_table, explicit_thread_migration, measure, run_all_scenarios, same_thread_suspend,
};

fn iterations_from_args() -> usize {
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--iterations" {
            return args
                .next()
                .and_then(|value| value.parse().ok())
                .unwrap_or(100);
        }
    }
    100
}

fn main() {
    run_all_scenarios().expect("07-D.6 原型场景失败");
    println!("scenarios=8 passed");
    let same_thread = same_thread_suspend();
    let migrated = explicit_thread_migration();
    println!(
        "cross_thread.same_thread_suspend=task:{} thread:{}",
        same_thread.task_boundary, same_thread.thread_boundary
    );
    println!(
        "cross_thread.explicit_migration=task:{} thread:{}",
        migrated.task_boundary, migrated.thread_boundary
    );
    for row in candidate_a_table() {
        println!("candidate_a.{}={}", row.label, row.accepted);
    }
    println!("{}", measure(iterations_from_args()));
}
