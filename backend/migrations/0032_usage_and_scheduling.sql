-- 请求明细与统计共用 1～31 天保留期，已有超限设置收敛到 31 天
alter table runtime_settings drop constraint runtime_settings_retention_ck;
alter table runtime_settings alter column usage_retention_days set default 31;
update runtime_settings set usage_retention_days = 31 where usage_retention_days > 31;
alter table runtime_settings add constraint runtime_settings_retention_ck check (
    usage_retention_days between 1 and 31
    and ops_event_retention_days > 0
    and audit_retention_days > 0
);

-- 删除与新增、修改共用异步投影，读取已发布统计直到 Worker 原子撤销旧贡献
-- 缓存代次只在统计发布时推进，源清理不使仍然一致的统计快照失效
drop trigger request_analytics_delete_invalidation on model_requests;
drop trigger request_analytics_ops_delete_invalidation on ops_events;
drop function invalidate_deleted_request_analytics();
drop function invalidate_deleted_ops_analytics();

create function enqueue_request_analytics(request_ref text)
returns void language sql as $$
    insert into request_analytics_pending_requests(request_id)
    select request_ref where request_ref is not null
    on conflict (request_id) do update
    set change_version = nextval('request_analytics_change_version')
$$;

create or replace function observe_request_analytics_change() returns trigger language plpgsql as $$
begin
    if TG_OP = 'DELETE' then
        perform enqueue_request_analytics(OLD.id);
        return OLD;
    end if;
    perform enqueue_request_analytics(NEW.id);
    return NEW;
end
$$;

create or replace function observe_ops_analytics_change() returns trigger language plpgsql as $$
begin
    if TG_OP <> 'INSERT' then
        perform enqueue_request_analytics(OLD.model_request_id);
    end if;
    if TG_OP <> 'DELETE' then
        perform enqueue_request_analytics(NEW.model_request_id);
        return NEW;
    end if;
    return OLD;
end
$$;

-- 旧时间桶由已发布事实定位，不再维护只用于阻断读取的删除标记与索引
drop function enqueue_request_analytics(text, timestamptz, boolean);
alter table request_analytics_pending_requests
    drop column previous_bucket,
    drop column deletion_pending;

-- 统一领域名称和统计时间粒度，仅重命名现有对象，不复制数据或重建索引
do $$
declare
    names record;
    constraint_name record;
    index_name record;
    table_oid oid;
begin
    for names in
        select * from (values
            ('client_key_budget_windows', 'client_api_key_budget_windows'),
            ('client_key_charge_events', 'client_api_key_charge_events'),
            ('authorization_receipts', 'provider_authorization_receipts'),
            ('plugin_group_resources', 'plugin_account_group_resources'),
            ('plugin_key_resources', 'plugin_client_api_key_resources'),
            ('request_analytics_hourly', 'request_analytics_1h'),
            ('request_analytics_daily_counts', 'request_analytics_1d_counts')
        ) as renamed(old_name, new_name)
    loop
        table_oid := format('%I', names.old_name)::regclass;
        execute format('alter table %I rename to %I', names.old_name, names.new_name);

        -- 主键和唯一约束改名会同步改名其索引；目录查询也覆盖数据库生成的约束名
        -- LIKE 复制的非空约束可能沿用十五分钟表的前缀，仅修正小时表自身的约束
        for constraint_name in
            select conname,
                case when starts_with(conname, names.old_name || '_')
                    then names.old_name else 'request_analytics_15m' end as prefix
            from pg_constraint
            where conrelid = table_oid
              and (starts_with(conname, names.old_name || '_')
                or (names.old_name = 'request_analytics_hourly'
                    and starts_with(conname, 'request_analytics_15m_')))
            order by conname
        loop
            execute format('alter table %I rename constraint %I to %I',
                names.new_name, constraint_name.conname,
                names.new_name || substr(constraint_name.conname, length(constraint_name.prefix) + 1));
        end loop;

        for index_name in
            select c.relname from pg_index i join pg_class c on c.oid = i.indexrelid
            where i.indrelid = table_oid and starts_with(c.relname, names.old_name || '_')
            order by c.relname
        loop
            execute format('alter index %I rename to %I', index_name.relname,
                names.new_name || substr(index_name.relname, length(names.old_name) + 1));
        end loop;
    end loop;
end
$$;

alter trigger request_analytics_hourly_daily_counts on request_analytics_1h
    rename to request_analytics_1h_1d_counts;
alter function sync_request_analytics_daily_counts() rename to sync_request_analytics_1d_counts;

-- PL/pgSQL 函数体中的表名不会随表重命名自动改写，保留原有日计数增减语义
create or replace function sync_request_analytics_1d_counts() returns trigger language plpgsql as $$
declare
    old_day timestamptz;
    new_day timestamptz;
    delta bigint;
begin
    if TG_OP <> 'INSERT' then
        old_day := date_bin(interval '1 day',OLD.bucket_start,timestamptz '2000-01-01 UTC');
    end if;
    if TG_OP <> 'DELETE' then
        new_day := date_bin(interval '1 day',NEW.bucket_start,timestamptz '2000-01-01 UTC');
    end if;
    if TG_OP = 'UPDATE' and
        (old_day,OLD.collection,OLD.dimensions[1],OLD.outcome,OLD.schema_version,OLD.histogram_version)
        is not distinct from
        (new_day,NEW.collection,NEW.dimensions[1],NEW.outcome,NEW.schema_version,NEW.histogram_version) then
        if NEW.collection in ('account','key') then
            delta := NEW.request_count - OLD.request_count;
            if delta <> 0 then
                update request_analytics_1d_counts set request_count=request_count+delta
                where collection=NEW.collection and bucket_start=new_day and dimension=NEW.dimensions[1]
                    and outcome=NEW.outcome and schema_version=NEW.schema_version and histogram_version=NEW.histogram_version;
            end if;
        end if;
        return null;
    end if;
    if TG_OP <> 'INSERT' and OLD.collection in ('account','key') then
        update request_analytics_1d_counts set request_count=request_count-OLD.request_count
        where collection=OLD.collection and bucket_start=old_day and dimension=OLD.dimensions[1]
            and outcome=OLD.outcome and schema_version=OLD.schema_version and histogram_version=OLD.histogram_version;
        delete from request_analytics_1d_counts
        where collection=OLD.collection and bucket_start=old_day and dimension=OLD.dimensions[1]
            and outcome=OLD.outcome and schema_version=OLD.schema_version and histogram_version=OLD.histogram_version
            and request_count=0;
    end if;
    if TG_OP <> 'DELETE' and NEW.collection in ('account','key') then
        insert into request_analytics_1d_counts values(new_day,NEW.collection,NEW.dimensions[1],NEW.outcome,NEW.schema_version,NEW.histogram_version,NEW.request_count)
        on conflict(collection,bucket_start,dimension,outcome,schema_version,histogram_version)
        do update set request_count=request_analytics_1d_counts.request_count+excluded.request_count;
    end if;
    return null;
end;
$$;

-- 分组生图权限默认允许，保留已有分组及未显式设置权限的创建行为
ALTER TABLE account_groups
    ADD COLUMN allow_image_generation boolean NOT NULL DEFAULT true;

-- 删除自动回切配置，保留既有智能调度系数
UPDATE runtime_settings
SET smart_scheduling_json = smart_scheduling_json - 'preferHigherWeight'
WHERE smart_scheduling_json ? 'preferHigherWeight';

ALTER TABLE runtime_settings
    ALTER COLUMN smart_scheduling_json SET DEFAULT
    '{"loadWeight":1.0,"quotaWeight":0.8,"healthWeight":1.0,"latencyWeight":0.5,"resetWeight":0.0,"queueWeight":0.0}'::jsonb;

-- 初始账号并发上限为 5，已保存的运行设置保持原值
alter table runtime_settings alter column max_concurrent_per_account set default 5;
update runtime_settings set max_concurrent_per_account = 5
where config_revision = 1 and max_concurrent_per_account = 3;
