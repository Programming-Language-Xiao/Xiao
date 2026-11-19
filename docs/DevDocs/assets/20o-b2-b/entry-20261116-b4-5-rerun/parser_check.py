"""验证完整workspace日志解析的同名目标、子进程和失败边界。"""
import sys
sys.dont_write_bytecode = True
import json
from record import require
from validation import workspace_counts


def main():
    fixture = """     Running tests/c05_tables.rs (D:/task/c05_tables-a01.exe)
test result: ok. 2 passed; 0 failed; 0 ignored;
     Running tests/c05_tables.rs (D:/task/c05_tables-b02.exe)
test result: ok. 3 passed; 0 failed; 0 ignored;
     Running tests/parent.rs (D:/task/parent-c03.exe)
test result: ok. 1 passed; 0 failed; 0 ignored;
test result: ok. 5 passed; 0 failed; 1 ignored;
   Doc-tests example
test result: ok. 0 passed; 0 failed; 0 ignored;
"""
    result = workspace_counts(fixture)
    require(result["targets"] == {"c05_tables@a01": [2, 0, 0], "c05_tables@b02": [3, 0, 0],
                                  "parent@c03": [5, 0, 1], "example:doc": [0, 0, 0]}
            and result["nested_summaries"] == [{"parent": "parent@c03", "counts": [1, 0, 0]}]
            and result["passed"] == 10 and result["ignored"] == 1, "workspace解析自测不符")
    for source, reason in [
        (fixture.replace("ok. 1 passed; 0 failed", "FAILED. 0 passed; 1 failed"),
         "B4-5workspace摘要缺失目标或包含失败"),
        (fixture + "     Running tests/missing.rs (D:/task/missing-d04.exe)\n",
         "B4-5workspace目标汇总缺失/重复"),
    ]:
        try:
            workspace_counts(source)
        except ValueError as error:
            require(str(error) == reason, "workspace解析拒绝原因不符")
        else:
            raise ValueError("workspace解析未拒绝异常")
    print(json.dumps({"passed": True, "duplicate_names_preserved": True, "nested_summaries_preserved": True,
                      "failed_child_rejected": True, "missing_summary_rejected": True}))


if __name__ == "__main__":
    main()
