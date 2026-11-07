"""重算F-3统计与原始协议，拒绝空队列、缺批、假校准、错误RSS单位或漏规模。"""
import argparse
import hashlib
import json
import math
from pathlib import Path
import statistics


def require(condition, reason):
    if not condition:
        raise SystemExit(reason)


def memory(value):
    require(isinstance(value, dict), 'OS内存读数不可验证')
    factor = {'KiB': 1024, 'bytes': 1}.get(value['unit'])
    require(factor is not None and value['current_raw'] > 0 and value['peak_raw'] >= value['current_raw'], 'OS原始内存值或单位不符')
    require(value['current_bytes'] == value['current_raw']*factor and value['peak_bytes'] == value['peak_raw']*factor, 'OS内存单位换算错误')
    expected = {'proc-vmrss-vmhwm': 'KiB', 'working-set/peak-working-set': 'bytes', 'resident-size/getrusage-ru-maxrss': 'bytes'}
    require(expected.get(value['method']) == value['unit'], 'OS内存来源与单位不匹配')
    return value


def local_path(original, original_root, artifact_root):
    normalized = original.replace('\\', '/')
    require(normalized.startswith(original_root+'/'), '原始产物引用越界')
    relative = normalized[len(original_root)+1:]
    result = (artifact_root/relative).resolve()
    require(result.is_relative_to(artifact_root.resolve()), '本地产物引用越界')
    return result


def original_path(value):
    normalized = value.replace('\\', '/')
    return normalized.removeprefix('//?/')


def artifacts(report):
    require(report.get('schema') == 1 and report.get('backend') in {'vm', 'native'}, '性能报告schema或后端不符')
    items = report.get('artifacts')
    require(isinstance(items, list) and bool(items), '实际产物集合缺项、重复或后端不符')
    require(all(isinstance(item, dict) and isinstance(item.get('path'), str) and item['path']
                for item in items), '实际产物集合缺项、重复或后端不符')
    suffix = '.exe' if report['identity']['target'].endswith('-windows-msvc') else ''
    expected = {'measurement.xiao', 'measurement.xiaoc'} if report['backend'] == 'vm' else {
        'measurement.xiao', 'numeric-o0'+suffix, 'numeric-o3'+suffix}
    names = [item['path'].replace('\\', '/').rsplit('/', 1)[-1] for item in items]
    require(len(names) == len(expected) and set(names) == expected, '实际产物集合缺项、重复或后端不符')
    return dict(zip(names, items))


def next_repeat(previous):
    # schema1控制器只依据上个原始时长确定下一批次数；不能另造一个满足增长上限的序列。
    factor = 16 if previous['seconds'] == 0 else max(2, min(16, math.ceil(0.1/previous['seconds'])))
    return min(previous['repeats']*factor, 1 << 24)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--report', type=Path, required=True)
    parser.add_argument('--artifacts', type=Path)
    parser.add_argument('--require-full', action='store_true')
    parser.add_argument('--catalog', type=Path)
    args = parser.parse_args()
    report = json.loads(args.report.read_text(encoding='utf-8'))
    actual_artifacts = artifacts(report)
    require(report['complete'] and report['source_unchanged'] and report['passed'], '报告未完成、失败或源码变化')
    require(report['runtime_abi'] == (1 << 16 | 18) and report['codegen'] == 20, '版本边界不符')
    require(report['identity']['rustc'].startswith('rustc 1.96.0'), 'Rust构建版本不符')
    protocol = report['protocol']
    require(protocol['batches'] == 5 and protocol['warmup_per_process'] == 1 and protocol['maximum_repeat'] == 1 << 24
            and protocol['minimum_batch_seconds'] == 0.05 and protocol['calibration_target_seconds'] == 0.1
            and protocol['maximum_growth_factor'] == 16 and protocol['memory_limit_bytes'] == 64*1024*1024, 'F-3协议数值漂移')
    original_root = actual_artifacts['measurement.xiao']['path'].replace('\\', '/').rsplit('/', 1)[0]
    artifact_root = args.artifacts or Path(original_root)
    for item in report['artifacts']:
        path = local_path(item['path'], original_root, artifact_root)
        raw = path.read_bytes()
        require(len(raw) == item['bytes'] and hashlib.sha256(raw).hexdigest() == item['sha256'], '实际产物摘要不符：'+path.name)
    catalog_file = args.catalog or Path(__file__).resolve().parents[5]/'tests/fixtures/20o-b2-a6/catalog.json'
    catalog = json.loads(catalog_file.read_text(encoding='utf-8'))
    require(catalog['D'] == 32768 and catalog['W'] == 65536 and catalog['H'] == 100000, '资源常量漂移')
    expected = {(case['id'], profile['label']): (case, profile) for case in catalog['cases'] for profile in case['profiles']
                if report['selected_case'] in ['all', case['id']] and report['selected_profile'] in ['all', profile['label']]}
    levels = [0] if report['backend'] == 'vm' else [0, 3]
    expected_keys = {(case, profile, level) for case, profile in expected for level in levels}
    keys = [(row['case'], row['profile'], row['optimization']) for row in report['records']]
    require(bool(keys) and len(keys) == len(set(keys)) and set(keys) == expected_keys, '实际负载、优化或规模缺项/重复')
    if args.require_full:
        require(report['full_catalog'] and report['full_catalog_passed'] and report['selected_case'] == report['selected_profile'] == 'all', '子集不可当作全量A6验收')
    medians = {}
    process_identities = set()
    for row in report['records']:
        require(row['status'] == 'passed' and row['error'] is None and len(row['batches']) == 5, '失败、缺批或不可验证规模')
        case, profile = expected[(row['case'], row['profile'])]
        require(row['size'] == profile['size'] and row['maximum'] == profile['maximum']
                and row['dimension'] == case['dimension'] and row['complexity'] == case['complexity'], '实际输入族/规模身份不符')
        times, peaks = [], []
        for index, batch in enumerate(row['batches']):
            require(batch['pid'] > 0 and batch['calibrated'], '缺少真实PID或未完成校准')
            base = memory(batch['baseline'])
            calibration = batch['calibration']
            require(bool(calibration) and calibration[0]['repeats'] == 1 and batch['sample'] == calibration[-1], '正式样本与全部校准记录不符')
            high = base['peak_bytes']
            previous = None
            for item in calibration:
                repeats, seconds = item['repeats'], item['seconds']
                require(isinstance(repeats, int) and 1 <= repeats <= 1 << 24 and math.isfinite(seconds) and seconds >= 0, '时长或重复次数无效')
                if previous:
                    require(previous['seconds'] < 0.1 and not (previous['repeats'] == 1 and previous['seconds'] >= 0.05)
                            and previous['repeats'] < 1 << 24 and previous['repeats'] < repeats <= min(previous['repeats']*16, 1 << 24), '校准增长或停止条件违反F-3')
                    require(repeats == next_repeat(previous), '重复次数与原始时长推导的实际校准流程不符')
                high = max(high, memory(item['memory'])['peak_bytes'])
                previous = item
            sample = batch['sample']
            require(sample['seconds'] >= 0.05 and (sample['repeats'] == 1 or sample['seconds'] >= 0.1 or sample['repeats'] == 1 << 24), '校准不足50ms或未按100ms停止')
            increment = max(0, high-base['current_bytes'])
            require(increment == batch['resident_increment_upper_bytes'], '单PID OS峰值增量重算不符')
            times.append(sample['seconds']/sample['repeats'])
            peaks.append(increment)
            stdout = local_path(batch['stdout'], original_root, artifact_root)
            events = [json.loads(line) for line in stdout.read_text(encoding='utf-8').splitlines()]
            require([event['event'] for event in events] == ['baseline', 'validated', 'ready']+['sample', 'validated']*len(calibration)+['done'], '原始协议缺少预热/验证或测量窗口')
            require(events[0]['pid'] == events[2]['pid'] == batch['pid'] and events[0]['memory'] == batch['baseline'], '原始基线PID/读数不符')
            for event, item in zip(events[3:-1:2], calibration):
                require(event['seconds'] == item['seconds'] and event['memory'] == item['memory'], '原始时长/内存与报告样本不符')
            require(not local_path(batch['stderr'], original_root, artifact_root).read_bytes(), '子进程stderr不为空')
            process_file = stdout.with_name(stdout.name.removesuffix('.stdout.jsonl')+'.process.json')
            process = json.loads(process_file.read_text(encoding='utf-8'))
            require(process['pid'] == batch['pid'] and process['batch'] == index and process['case'] == row['case'] and process['profile'] == row['profile'], '原始进程身份不符')
            relative = f"optimization-{row['optimization']}/{row['case']}-{row['profile']}-batch{index}"
            expected_stdout = artifact_root/(relative+'.stdout.jsonl')
            expected_stderr = artifact_root/(relative+'.stderr.log')
            require(stdout == expected_stdout.resolve()
                    and local_path(batch['stderr'], original_root, artifact_root) == expected_stderr.resolve()
                    and isinstance(process.get('stdout'), str) and isinstance(process.get('stderr'), str)
                    and original_path(process['stdout']) == original_path(batch['stdout'])
                    and original_path(process['stderr']) == original_path(batch['stderr']), '原始进程输出路径或规模关联不符')
            started = batch.get('started_unix_nanoseconds')
            require(type(started) is int and started > 0 and process.get('started_unix_nanoseconds') == str(started), '原始进程启动时间不符')
            identity = (batch['pid'], started)
            require(identity not in process_identities, '测量样本重复使用同一次进程')
            process_identities.add(identity)
            if report['backend'] == 'vm':
                executable = report['identity']['worker']['path']
                expected_args = ['--worker-vm', actual_artifacts['measurement.xiaoc']['path']]
            else:
                suffix = '.exe' if report['identity']['target'].endswith('-windows-msvc') else ''
                executable = actual_artifacts[f"numeric-o{row['optimization']}"+suffix]['path']
                expected_args = []
            require(isinstance(process.get('executable'), str) and bool(process['executable'])
                    and isinstance(executable, str) and bool(executable)
                    and original_path(process['executable']) == original_path(executable)
                    and process.get('argv') == expected_args, '原始进程执行文件或参数未绑定实际产物')
        median = statistics.median(times)
        require(times == row['normalized_seconds'] and median == row['median_seconds'] and max(peaks) == row['resident_increment_upper_bytes'], '五批归一化中位数/保守峰值重算不符')
        limit = 10 if report['backend'] == 'vm' else 2
        require(row['seconds_limit'] == limit and median <= limit and max(peaks) <= 64*1024*1024, '原始数据未达时间或OS峰值预算')
        medians[(row['case'], row['profile'], row['optimization'])] = median
    for row in report['records']:
        limit = 3 if row['complexity'] == 'linear' else 5
        require(row['ratio_limit'] == limit, '翻倍预算漂移')
        if row['profile'] == '2n' and (row['case'], 'n', row['optimization']) in medians:
            ratio = row['median_seconds']/medians[(row['case'], 'n', row['optimization'])]
            require(ratio == row['ratio'] and ratio <= limit, '同路径翻倍比重算未通过')
    print(json.dumps({'passed': True, 'backend': report['backend'], 'records': len(keys), 'independent_batches': len(keys)*5,
                      'full_catalog': report['full_catalog']}, ensure_ascii=False))


if __name__ == '__main__':
    main()
