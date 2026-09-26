/** fish 用函数保存和恢复原提示符，激活数据仍遵循 E2B 的两行白名单。 */
export function fishShellInitScript(commandName: string): string {
  return String.raw`# xiao shell-init fish
function __xiao_activate_environment -a name use_color path
    if not set -q __xiao_prompt_saved
        if functions -q fish_prompt
            functions -c fish_prompt __xiao_original_prompt
        end
        set -g __xiao_prompt_saved 1
    end
    set -g __xiao_prompt_name "$name"
    set -g __xiao_prompt_color "$use_color"
    set -gx XIAO_ACTIVE_ENV "$path"
    function fish_prompt
        if test "$__xiao_prompt_color" = 1
            set_color green
        end
        printf '$%s$ ' "$__xiao_prompt_name"
        if test "$__xiao_prompt_color" = 1
            set_color normal
        end
        if functions -q __xiao_original_prompt
            __xiao_original_prompt
        end
    end
end
function __xiao_deactivate_environment
    if set -q __xiao_prompt_saved
        functions -e fish_prompt
        if functions -q __xiao_original_prompt
            functions -c __xiao_original_prompt fish_prompt
            functions -e __xiao_original_prompt
        end
        set -e __xiao_prompt_saved __xiao_prompt_name __xiao_prompt_color
    end
    set -e XIAO_ACTIVE_ENV
end
function ${commandName}
    set -l _xiao_command ''
    set -l _xiao_override 0
    for _xiao_arg in $argv
        switch "$_xiao_arg"
            case --help -h --version -v
                set _xiao_override 1
            case --json -debug '--color=*'
            case '*'
                if test -z "$_xiao_command"
                    set _xiao_command "$_xiao_arg"
                end
        end
    end
    if test $_xiao_override -eq 1
        set _xiao_command ''
    end
    set -l _xiao_base /tmp
    if set -q TMPDIR
        set _xiao_base "$TMPDIR"
    end
    if command -sq cygpath; and set -q TEMP
        set _xiao_base (cygpath -u "$TEMP"); or return 70
    end
    set -l _xiao_dir (mktemp -d "$_xiao_base/xiao-activation.XXXXXXXX"); or return 70
    set -l _xiao_file (mktemp "$_xiao_dir/activation.XXXXXXXX")
    or begin
        rmdir -- "$_xiao_dir"
        return 70
    end
    if command -sq cygpath
        set -gx XIAO_ACTIVATION_FILE (cygpath -w "$_xiao_file")
    else
        set -gx XIAO_ACTIVATION_FILE "$_xiao_file"
    end
    command ${commandName} $argv
    set -l _xiao_status $status
    set -e XIAO_ACTIVATION_FILE
    if test $_xiao_status -eq 0; and contains -- "$_xiao_command" venv sync
        set -l _xiao_lines
        while read -l _xiao_line; or test -n "$_xiao_line"
            set -a _xiao_lines "$_xiao_line"
        end < "$_xiao_file"
        if test (count $_xiao_lines) -eq 2; and test "$_xiao_lines[2]" = 'export XIAO_ACTIVE_ENV'; and string match -r -q "^XIAO_ACTIVE_ENV='[^']+'\$" -- "$_xiao_lines[1]"
            set -l _xiao_path (string replace -r "^XIAO_ACTIVE_ENV='([^']+)'\$" '$1' -- "$_xiao_lines[1]")
            if not string match -r -q '[[:cntrl:]]' -- "$_xiao_path"; and string match -r -q '^(/|[A-Za-z]:[/\\\\])' -- "$_xiao_path"
                set -l _xiao_normalized (string replace -a '\\' '/' -- "$_xiao_path")
                set -l _xiao_name (basename -- "$_xiao_normalized")
                if test "$_xiao_name" = .venv
                    set _xiao_name venv
                end
                set -l _xiao_can_color 0
                if status is-interactive; and test -t 1; and not set -q NO_COLOR; and test "$TERM" != dumb
                    set _xiao_can_color 1
                end
                set -l _xiao_color $_xiao_can_color
                for _xiao_arg in $argv
                    if test "$_xiao_arg" = --color=never
                        set _xiao_color 0
                    else if test "$_xiao_arg" = --color=always; and test $_xiao_can_color -eq 1
                        set _xiao_color 1
                    end
                end
                __xiao_activate_environment "$_xiao_name" "$_xiao_color" "$_xiao_path"
            end
        end
    else if test $_xiao_status -eq 0; and test "$_xiao_command" = deactivate
        __xiao_deactivate_environment
    end
    rm -f -- "$_xiao_file"
    rmdir -- "$_xiao_dir"
    return $_xiao_status
end
`;
}
