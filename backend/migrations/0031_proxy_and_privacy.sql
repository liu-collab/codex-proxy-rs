-- 命名代理独立启停，保留连接配置和账号绑定
alter table outbound_proxies add column enabled boolean not null default true;

-- 非对象规则不能无损转换，拒绝升级以保留原始配置
do $$
begin
    if exists (
        select 1 from runtime_settings,
            jsonb_array_elements(codex_privacy_policy_json->'rules') as rules(rule)
        where jsonb_typeof(rule) <> 'object'
    ) then
        raise exception 'privacy rules must be objects' using errcode = '23514';
    end if;
end $$;

-- 逐条迁移，不合并规则，保留原有开关、执行顺序和失败边界
update runtime_settings
set codex_privacy_policy_json = jsonb_set(codex_privacy_policy_json, '{rules}', (
    select jsonb_agg(
        case when rule ? 'steps' then rule
        else jsonb_build_object(
            'id', rule->'id',
            'name', rule->'name',
            'enabled', rule->'enabled',
            'steps', jsonb_build_array(rule - array['id', 'name', 'enabled'])
        ) end order by position
    )
    from jsonb_array_elements(codex_privacy_policy_json->'rules')
        with ordinality as rules(rule, position)
))
where exists (
    select 1 from jsonb_array_elements(codex_privacy_policy_json->'rules') as rules(rule)
    where not rule ? 'steps'
);
