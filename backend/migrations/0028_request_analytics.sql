-- 请求历史身份、观测校验、恢复租约与可重建统计
-- 迁移在停止旧写入者的窗口执行，旧二进制不可直接回退

create or replace function model_request_observation_valid(document jsonb) returns boolean
language plpgsql immutable parallel safe as $$
declare
  spec record;
  item jsonb;
  number numeric;
begin
  if document is null or jsonb_typeof(document) <> 'object'
     or octet_length(document::text) > 1048576 then
    return false;
  end if;
  for spec in select * from (values
    ('{}'::text[], array['account','continuation','error','recovery','request','response','routing','scheduling','timings','transport']::text[]),
    ('{request}'::text[], array['clientApiKeyNameSnapshot','clientIp','compact','configRevision','endpoint','protocol','reasoningEffort','reasoningPreset','subagentKind','userAgent']::text[]),
    ('{account}'::text[], array['authenticationKind','email','name','planTypeSnapshot','notesSnapshot']::text[]),
    ('{routing}'::text[], array['groupNamesSnapshot','groupRefs','scope']::text[]),
    ('{error}'::text[], array['message','providerErrorCode','retryAfterMs']::text[]),
    ('{timings}'::text[], array['local','upstream']::text[]),
    ('{timings,local}'::text[], array['connectMs','firstEventMs','firstReasoningMs','firstTextMs','firstTokenMs','headersMs','latencyMs','transportDecisionWaitMs']::text[]),
    ('{timings,upstream}'::text[], array['processingMs','responseMs','apiOverheadMs','engineMs','engineIapiTtftMs','engineServiceTtftMs','engineIapiTbtMs','engineServiceTbtMs']::text[]),
    ('{scheduling}'::text[], array['accountSelectionWaitMs','admissionDecisionMs','capacityTotalSlots','capacityUsedSlots']::text[]),
    ('{transport}'::text[], array['connection','httpVersion','websocketPool']::text[]),
    ('{transport,connection}'::text[], array['ageMs','exitReason','id','idleMs']::text[]),
    ('{response}'::text[], array['responseModel']::text[]),
    ('{continuation}'::text[], array['previousResponseIdHash','unavailableReason']::text[]),
    ('{recovery}'::text[], array['retryDelayMs','totalLatencyMs']::text[])) as groups(path, allowed_keys) loop
    item := document #> spec.path;
    if item is not null and (jsonb_typeof(item) <> 'object' or item - spec.allowed_keys <> '{}'::jsonb) then
      return false;
    end if;
  end loop;
  for spec in select * from (values
    ('{request,configRevision}'::text[], 'number'),
    ('{request,protocol}'::text[], 'string'),
    ('{request,endpoint}'::text[], 'string'),
    ('{request,clientApiKeyNameSnapshot}'::text[], 'string'),
    ('{account,planTypeSnapshot}'::text[], 'string'),
    ('{account,notesSnapshot}'::text[], 'string'),
    ('{account,name}'::text[], 'string'),
    ('{account,email}'::text[], 'string'),
    ('{account,authenticationKind}'::text[], 'string'),
    ('{routing,scope}'::text[], 'string'),
    ('{routing,groupRefs}'::text[], 'array'),
    ('{routing,groupNamesSnapshot}'::text[], 'array'),
    ('{error,providerErrorCode}'::text[], 'string'),
    ('{error,message}'::text[], 'string'),
    ('{error,retryAfterMs}'::text[], 'number'),
    ('{request,clientIp}'::text[], 'string'),
    ('{request,userAgent}'::text[], 'string'),
    ('{request,reasoningEffort}'::text[], 'string'),
    ('{request,reasoningPreset}'::text[], 'string'),
    ('{request,subagentKind}'::text[], 'string'),
    ('{request,compact}'::text[], 'boolean'),
    ('{timings,local,transportDecisionWaitMs}'::text[], 'number'),
    ('{timings,local,connectMs}'::text[], 'number'),
    ('{timings,local,headersMs}'::text[], 'number'),
    ('{timings,local,firstEventMs}'::text[], 'number'),
    ('{timings,local,firstReasoningMs}'::text[], 'number'),
    ('{timings,local,firstTextMs}'::text[], 'number'),
    ('{timings,local,firstTokenMs}'::text[], 'number'),
    ('{timings,local,latencyMs}'::text[], 'number'),
    ('{timings,upstream,processingMs}'::text[], 'number'),
    ('{timings,upstream,apiOverheadMs}'::text[], 'milliseconds'),
    ('{timings,upstream,engineMs}'::text[], 'milliseconds'),
    ('{timings,upstream,engineIapiTtftMs}'::text[], 'milliseconds'),
    ('{timings,upstream,engineServiceTtftMs}'::text[], 'milliseconds'),
    ('{timings,upstream,engineIapiTbtMs}'::text[], 'milliseconds'),
    ('{timings,upstream,engineServiceTbtMs}'::text[], 'milliseconds'),
    ('{scheduling,admissionDecisionMs}'::text[], 'number'),
    ('{scheduling,accountSelectionWaitMs}'::text[], 'number'),
    ('{scheduling,capacityUsedSlots}'::text[], 'number'),
    ('{scheduling,capacityTotalSlots}'::text[], 'number'),
    ('{transport,httpVersion}'::text[], 'string'),
    ('{transport,websocketPool}'::text[], 'string'),
    ('{transport,connection,id}'::text[], 'string'),
    ('{transport,connection,exitReason}'::text[], 'string'),
    ('{transport,connection,ageMs}'::text[], 'number'),
    ('{transport,connection,idleMs}'::text[], 'number'),
    ('{response,responseModel}'::text[], 'string'),
    ('{continuation,previousResponseIdHash}'::text[], 'string'),
    ('{continuation,unavailableReason}'::text[], 'string'),
    ('{recovery,retryDelayMs}'::text[], 'number'),
    ('{recovery,totalLatencyMs}'::text[], 'number'),
    ('{timings,upstream,responseMs}'::text[], 'number')) as fields(path, kind) loop
    item := document #> spec.path;
    if item is null or item = 'null'::jsonb then continue; end if;
    if spec.kind = 'milliseconds' then
      if jsonb_typeof(item) <> 'number' then return false; end if;
      number := (item #>> '{}')::numeric;
      if number < 0 or number > 9223372036854775807 then return false; end if;
      continue;
    end if;
    if jsonb_typeof(item) <> spec.kind then return false; end if;
    if spec.kind = 'number' then
      number := (item #>> '{}')::numeric;
      if item #>> '{}' !~ '^[0-9]+$' or number > 9223372036854775807 then return false; end if;
    elsif spec.kind = 'array' then
      if exists (select 1 from jsonb_array_elements(item) member where jsonb_typeof(member) <> 'string') then return false; end if;
    end if;
  end loop;
  if (document #> '{request,configRevision}' is not null
          and document #> '{request,configRevision}' <> 'null'::jsonb
          and document #> '{request,compact}' in ('true'::jsonb, 'false'::jsonb)
          and coalesce(length(document #>> '{request,protocol}'), 0) > 0
          and coalesce(length(document #>> '{request,endpoint}'), 0) > 0
          and jsonb_typeof(document #> '{routing,scope}') = 'string'
          and jsonb_typeof(document #> '{routing,groupRefs}') = 'array'
          and jsonb_typeof(document #> '{routing,groupNamesSnapshot}') = 'array') is not true then
    return false;
  end if;
  if document #>> '{request,clientIp}' is not null then
    perform (document #>> '{request,clientIp}')::inet;
  end if;
  return true;
exception when invalid_text_representation or numeric_value_out_of_range then
  return false;
end;
$$;

-- 按最终行的真实依赖执行校验，纯状态写入不展开无关 JSON
create function check_model_request_observation() returns trigger
language plpgsql as $$
declare
  json_changed boolean := tg_op = 'INSERT' or old.request_observation_json is distinct from new.request_observation_json;
begin
  if json_changed then
    if model_request_observation_valid(new.request_observation_json) is not true then
      raise exception using errcode = '23514', constraint = 'model_requests_observation_shape_ck',
        message = 'request observation violates shape constraint', table = tg_table_name, schema = tg_table_schema;
    end if;
  end if;
  if json_changed then
    declare
      timings_upstream_response_ms bigint;
    begin
      timings_upstream_response_ms := (new.request_observation_json #>> '{timings,upstream,responseMs}')::bigint;
      if (timings_upstream_response_ms > 0) is false then
        raise exception using errcode = '23514', constraint = 'model_requests_upstream_duration_ck',
          message = 'request observation violates model_requests_upstream_duration_ck', table = tg_table_name, schema = tg_table_schema;
      end if;
    end;
  end if;
  if json_changed then
    if ((new.request_observation_json #>> '{request,configRevision}')::bigint > 0) is false then
      raise exception using errcode = '23514', constraint = 'model_requests_revision_attempt_ck', message = 'request observation violates model_requests_revision_attempt_ck', table = tg_table_name, schema = tg_table_schema;
    end if;
  end if;
  if json_changed then
    if ((new.request_observation_json #>> '{error,retryAfterMs}')::bigint is null or (new.request_observation_json #>> '{error,retryAfterMs}')::bigint >= 0) is false then
      raise exception using errcode = '23514', constraint = 'model_requests_status_ck', message = 'request observation violates model_requests_status_ck', table = tg_table_name, schema = tg_table_schema;
    end if;
  end if;
  if json_changed then
    declare
      transport_websocket_pool text;
    begin
      transport_websocket_pool := (new.request_observation_json #>> '{transport,websocketPool}');
      if (transport_websocket_pool is null or transport_websocket_pool in ('new', 'reuse')) is false then
        raise exception using errcode = '23514', constraint = 'model_requests_websocket_pool_ck',
          message = 'request observation violates model_requests_websocket_pool_ck', table = tg_table_name, schema = tg_table_schema;
      end if;
    end;
  end if;
  if json_changed then
    declare
      routing_group_names_snapshot jsonb;
      routing_scope text;
      routing_group_refs jsonb;
    begin
      routing_group_names_snapshot := (new.request_observation_json #> '{routing,groupNamesSnapshot}');
      routing_scope := (new.request_observation_json #>> '{routing,scope}');
      routing_group_refs := (new.request_observation_json #> '{routing,groupRefs}');
      if (jsonb_typeof(routing_group_names_snapshot) = 'array'
    and (
      (
        routing_scope in ('legacy_provider', 'all')
        and jsonb_array_length(routing_group_refs) = 0
        and jsonb_array_length(routing_group_names_snapshot) = 0
      )
      or (
        routing_scope = 'groups'
        and jsonb_array_length(routing_group_refs) > 0
        and not jsonb_path_exists(new.request_observation_json, '$.routing.groupRefs[*] ? (@ == null)')
        and jsonb_array_length(routing_group_names_snapshot)
          = jsonb_array_length(routing_group_refs)
      )
    )) is false then
        raise exception using errcode = '23514', constraint = 'model_requests_routing_group_names_ck',
          message = 'request observation violates model_requests_routing_group_names_ck', table = tg_table_name, schema = tg_table_schema;
      end if;
    end;
  end if;
  if json_changed then
    declare
      routing_scope text;
    begin
      routing_scope := (new.request_observation_json #>> '{routing,scope}');
      if (routing_scope in ('legacy_provider', 'all', 'groups')) is false then
        raise exception using errcode = '23514', constraint = 'model_requests_routing_scope_ck',
          message = 'request observation violates model_requests_routing_scope_ck', table = tg_table_name, schema = tg_table_schema;
      end if;
    end;
  end if;
  if json_changed then
    declare
      timings_local_transport_decision_wait_ms bigint;
      timings_local_connect_ms bigint;
      timings_local_headers_ms bigint;
      timings_local_first_event_ms bigint;
      timings_local_first_reasoning_ms bigint;
      timings_local_first_text_ms bigint;
      timings_local_first_token_ms bigint;
      timings_upstream_processing_ms bigint;
      timings_local_latency_ms bigint;
    begin
      timings_local_transport_decision_wait_ms := (new.request_observation_json #>> '{timings,local,transportDecisionWaitMs}')::bigint;
      timings_local_connect_ms := (new.request_observation_json #>> '{timings,local,connectMs}')::bigint;
      timings_local_headers_ms := (new.request_observation_json #>> '{timings,local,headersMs}')::bigint;
      timings_local_first_event_ms := (new.request_observation_json #>> '{timings,local,firstEventMs}')::bigint;
      timings_local_first_reasoning_ms := (new.request_observation_json #>> '{timings,local,firstReasoningMs}')::bigint;
      timings_local_first_text_ms := (new.request_observation_json #>> '{timings,local,firstTextMs}')::bigint;
      timings_local_first_token_ms := (new.request_observation_json #>> '{timings,local,firstTokenMs}')::bigint;
      timings_upstream_processing_ms := (new.request_observation_json #>> '{timings,upstream,processingMs}')::bigint;
      timings_local_latency_ms := (new.request_observation_json #>> '{timings,local,latencyMs}')::bigint;
      if ((timings_local_transport_decision_wait_ms is null or timings_local_transport_decision_wait_ms >= 0)
    and (timings_local_connect_ms is null or timings_local_connect_ms >= 0)
    and (timings_local_headers_ms is null or timings_local_headers_ms >= 0)
    and (timings_local_first_event_ms is null or timings_local_first_event_ms >= 0)
    and (timings_local_first_reasoning_ms is null or timings_local_first_reasoning_ms >= 0)
    and (timings_local_first_text_ms is null or timings_local_first_text_ms >= 0)
    and (timings_local_first_token_ms is null or timings_local_first_token_ms >= 0)
    and (timings_upstream_processing_ms is null or timings_upstream_processing_ms >= 0)
    and (timings_local_latency_ms is null or timings_local_latency_ms >= 0)
    and (
      timings_local_latency_ms is null
      or (
        (timings_local_transport_decision_wait_ms is null or timings_local_transport_decision_wait_ms <= timings_local_latency_ms)
        and (timings_local_connect_ms is null or timings_local_connect_ms <= timings_local_latency_ms)
        and (timings_local_headers_ms is null or timings_local_headers_ms <= timings_local_latency_ms)
        and (timings_local_first_event_ms is null or timings_local_first_event_ms <= timings_local_latency_ms)
        and (timings_local_first_reasoning_ms is null or timings_local_first_reasoning_ms <= timings_local_latency_ms)
        and (timings_local_first_text_ms is null or timings_local_first_text_ms <= timings_local_latency_ms)
        and (timings_local_first_token_ms is null or timings_local_first_token_ms <= timings_local_latency_ms)
      )
    )) is false then
        raise exception using errcode = '23514', constraint = 'model_requests_latency_ck',
          message = 'request observation violates model_requests_latency_ck', table = tg_table_name, schema = tg_table_schema;
      end if;
    end;
  end if;
  if json_changed then
    declare
      scheduling_admission_decision_ms bigint;
      scheduling_account_selection_wait_ms bigint;
      scheduling_capacity_used_slots bigint;
      scheduling_capacity_total_slots bigint;
    begin
      scheduling_admission_decision_ms := (new.request_observation_json #>> '{scheduling,admissionDecisionMs}')::bigint;
      scheduling_account_selection_wait_ms := (new.request_observation_json #>> '{scheduling,accountSelectionWaitMs}')::bigint;
      scheduling_capacity_used_slots := (new.request_observation_json #>> '{scheduling,capacityUsedSlots}')::bigint;
      scheduling_capacity_total_slots := (new.request_observation_json #>> '{scheduling,capacityTotalSlots}')::bigint;
      if ((scheduling_admission_decision_ms is null or scheduling_admission_decision_ms >= 0)
    and (scheduling_account_selection_wait_ms is null or scheduling_account_selection_wait_ms >= 0)
    and (
      (scheduling_capacity_used_slots is null and scheduling_capacity_total_slots is null)
      or (
        scheduling_capacity_used_slots >= 0
        and scheduling_capacity_total_slots > 0
        and scheduling_capacity_used_slots <= scheduling_capacity_total_slots
      )
    )) is false then
        raise exception using errcode = '23514', constraint = 'model_requests_runtime_pressure_ck',
          message = 'request observation violates model_requests_runtime_pressure_ck', table = tg_table_name, schema = tg_table_schema;
      end if;
    end;
  end if;
  if json_changed or old.continuation_requested is distinct from new.continuation_requested then
    if (((new.request_observation_json #>> '{continuation,previousResponseIdHash}') is null or (new.request_observation_json #>> '{continuation,previousResponseIdHash}') ~ '^[0-9a-f]{64}$') and (new.continuation_requested or (new.request_observation_json #>> '{continuation,previousResponseIdHash}') is null)) is false then
      raise exception using errcode = '23514', constraint = 'model_requests_continuation_hashes_ck', message = 'request observation violates model_requests_continuation_hashes_ck', table = tg_table_name, schema = tg_table_schema;
    end if;
  end if;
  if json_changed then
    declare
      continuation_unavailable_reason text;
    begin
      continuation_unavailable_reason := (new.request_observation_json #>> '{continuation,unavailableReason}');
      if (continuation_unavailable_reason is null
    or (
      octet_length(continuation_unavailable_reason) between 1 and 64
      and continuation_unavailable_reason ~ '^[a-z][a-z0-9_]*$'
    )) is false then
        raise exception using errcode = '23514', constraint = 'model_requests_continuation_reason_ck',
          message = 'request observation violates model_requests_continuation_reason_ck', table = tg_table_name, schema = tg_table_schema;
      end if;
    end;
  end if;
  if json_changed then
    declare
      transport_connection_id text;
      transport_connection_exit_reason text;
      transport_connection_age_ms bigint;
      transport_connection_idle_ms bigint;
    begin
      transport_connection_id := (new.request_observation_json #>> '{transport,connection,id}');
      transport_connection_exit_reason := (new.request_observation_json #>> '{transport,connection,exitReason}');
      transport_connection_age_ms := (new.request_observation_json #>> '{transport,connection,ageMs}')::bigint;
      transport_connection_idle_ms := (new.request_observation_json #>> '{transport,connection,idleMs}')::bigint;
      if ((
      transport_connection_id is null
      and transport_connection_exit_reason is null
      and transport_connection_age_ms is null
      and transport_connection_idle_ms is null
    )
    or (
      octet_length(transport_connection_id) between 1 and 128
      and transport_connection_id !~ '[[:cntrl:]]'
      and octet_length(transport_connection_exit_reason) between 1 and 64
      and transport_connection_exit_reason ~ '^[a-z][a-z0-9_]*$'
      and transport_connection_age_ms >= 0
      and transport_connection_idle_ms between 0 and transport_connection_age_ms
    )) is false then
        raise exception using errcode = '23514', constraint = 'model_requests_upstream_connection_ck',
          message = 'request observation violates model_requests_upstream_connection_ck', table = tg_table_name, schema = tg_table_schema;
      end if;
    end;
  end if;
  if json_changed or old.completed_at is distinct from new.completed_at or old.recovered_at is distinct from new.recovered_at or old.recovery_attempt_count is distinct from new.recovery_attempt_count or old.recovery_request_id is distinct from new.recovery_request_id then
    declare
      recovery_retry_delay_ms bigint;
      recovery_total_latency_ms bigint;
    begin
      recovery_retry_delay_ms := (new.request_observation_json #>> '{recovery,retryDelayMs}')::bigint;
      recovery_total_latency_ms := (new.request_observation_json #>> '{recovery,totalLatencyMs}')::bigint;
      if (new.recovery_attempt_count >= 0
    and (
      (
        new.recovered_at is null
        and new.recovery_request_id is null
        and recovery_retry_delay_ms is null
        and recovery_total_latency_ms is null
      )
      or (
        new.recovered_at is not null
        and new.recovered_at >= new.completed_at
        and new.recovery_request_id is not null
        and octet_length(new.recovery_request_id) between 1 and 128
        and new.recovery_attempt_count > 0
        and recovery_retry_delay_ms >= 0
        and recovery_total_latency_ms >= recovery_retry_delay_ms
      )
    )) is false then
        raise exception using errcode = '23514', constraint = 'model_requests_recovery_ck',
          message = 'request observation violates model_requests_recovery_ck', table = tg_table_name, schema = tg_table_schema;
      end if;
    end;
  end if;
  if json_changed then
    if (num_nonnulls(new.request_observation_json #>> '{scheduling,capacityUsedSlots}', new.request_observation_json #>> '{scheduling,capacityTotalSlots}') in (0, 2)
        and num_nonnulls(new.request_observation_json #>> '{transport,connection,id}', new.request_observation_json #>> '{transport,connection,exitReason}',
          new.request_observation_json #>> '{transport,connection,ageMs}', new.request_observation_json #>> '{transport,connection,idleMs}') in (0, 4)) is false then
        raise exception using errcode = '23514', constraint = 'model_requests_fact_completeness_ck',
          message = 'request observation violates model_requests_fact_completeness_ck', table = tg_table_name, schema = tg_table_schema;

    end if;
  end if;
  if json_changed or old.recovered_at is distinct from new.recovered_at or old.completed_at is distinct from new.completed_at then
    if (new.recovered_at is null or (new.completed_at is not null
        and new.request_observation_json #>> '{recovery,retryDelayMs}' is not null
        and new.request_observation_json #>> '{recovery,totalLatencyMs}' is not null)) is false then
        raise exception using errcode = '23514', constraint = 'model_requests_fact_completeness_ck',
          message = 'request observation violates model_requests_fact_completeness_ck', table = tg_table_name, schema = tg_table_schema;

    end if;
  end if;
  return null;
end;
$$;

create trigger model_requests_observation_insert
  after insert on model_requests
  for each row execute function check_model_request_observation();
create trigger model_requests_observation_update
  after update on model_requests for each row
  when (old.request_observation_json is distinct from new.request_observation_json
    or old.completed_at is distinct from new.completed_at
    or old.continuation_requested is distinct from new.continuation_requested
    or old.recovered_at is distinct from new.recovered_at
    or old.recovery_attempt_count is distinct from new.recovery_attempt_count
    or old.recovery_request_id is distinct from new.recovery_request_id
) execute function check_model_request_observation();

alter table model_requests
  drop constraint model_requests_observation_shape_ck,
  drop constraint model_requests_upstream_duration_ck,
  drop constraint model_requests_revision_attempt_ck,
  drop constraint model_requests_status_ck,
  drop constraint model_requests_websocket_pool_ck,
  drop constraint model_requests_routing_scope_ck,
  drop constraint model_requests_routing_group_names_ck,
  drop constraint model_requests_latency_ck,
  drop constraint model_requests_runtime_pressure_ck,
  drop constraint model_requests_continuation_hashes_ck,
  drop constraint model_requests_continuation_reason_ck,
  drop constraint model_requests_upstream_connection_ck,
  drop constraint model_requests_recovery_ck,
  drop constraint model_requests_fact_completeness_ck;

-- 不读取 JSON 的廉价约束保留为 CHECK，也保持与其他标量 CHECK 的错误优先级
alter table model_requests
  add constraint model_requests_revision_attempt_ck check (attempt_count >= 0),
  add constraint model_requests_status_ck check (
    (client_status_code is null or client_status_code between 100 and 599)
    and (upstream_status_code is null or upstream_status_code between 100 and 599)),
  add constraint model_requests_continuation_hashes_ck check (
    continuation_affinity_hash is null or continuation_affinity_hash ~ '^[0-9a-f]{64}$'),
  add constraint model_requests_recovery_ck check (recovery_attempt_count >= 0),
  add constraint model_requests_fact_completeness_ck check (
    cost_source = 'unavailable' or cost_currency is not null);

-- 历史引用与快照不依赖实体是否仍存在
drop view model_request_observations;
alter table model_requests
  drop constraint model_requests_client_fk,
  drop constraint model_requests_account_fk,
  drop constraint model_requests_client_ref_ck,
  drop constraint model_requests_account_ref_ck,
  drop column client_api_key_id,
  drop column provider_account_id,
  add constraint model_requests_client_ref_ck check (octet_length(client_api_key_ref) between 1 and 128),
  add constraint model_requests_account_ref_ck check (provider_account_ref is null or octet_length(provider_account_ref) between 1 and 128);
alter table ops_events
  drop constraint ops_events_account_fk,
  drop constraint ops_events_account_ref_ck,
  drop constraint ops_events_account_snapshot_present_ck,
  drop column provider_account_id,
  add column provider_account_plan_type_snapshot text,
  add constraint ops_events_account_ref_ck check (provider_account_ref is null or octet_length(provider_account_ref) between 1 and 128);

-- 租约只保存运行态，历史表不承担高频续期更新
create table model_request_leases (
  request_id text primary key references model_requests(id) on delete cascade,
  lease_expires_at timestamptz not null
);
create index model_request_leases_expiry_idx on model_request_leases (lease_expires_at, request_id);
insert into model_request_leases (request_id, lease_expires_at)
select id, deadline_at from model_requests where outcome = 'running';
drop index model_requests_running_deadline_idx;
alter table model_requests rename column deadline_at to recovery_lease_expires_at;
alter table model_requests alter column recovery_lease_expires_at drop not null;
update model_requests set recovery_lease_expires_at = null where outcome = 'running';

-- 主请求锁先于租约锁，终结和恢复共用同一顺序；续期只取得租约锁
create function finish_model_request_lease() returns trigger
language plpgsql as $$
declare
  expires_at timestamptz;
begin
  if old.outcome = 'running' and new.outcome <> 'running' then
    select lease_expires_at into expires_at from model_request_leases
      where request_id = new.id for update;
    new.recovery_lease_expires_at := coalesce(expires_at, old.recovery_lease_expires_at, old.started_at);
    delete from model_request_leases where request_id = new.id;
  end if;
  return new;
end;
$$;
create trigger model_requests_finish_lease before update on model_requests
  for each row when (old.outcome = 'running' and new.outcome <> 'running')
  execute function finish_model_request_lease();

-- 只更名，不改变 Token 的包含关系或费用语义
alter table model_requests rename column cached_tokens to cached_input_tokens;
alter table model_requests rename column cache_write_tokens to cache_write_input_tokens;
alter table model_requests rename column reasoning_tokens to reasoning_output_tokens;

create view model_request_observations as
select mr.*,
       coalesce(lease.lease_expires_at, mr.recovery_lease_expires_at) as effective_recovery_lease_expires_at,
       (mr.request_observation_json #>> '{request,configRevision}')::bigint as config_revision,
       (mr.request_observation_json #>> '{request,protocol}') as protocol,
       (mr.request_observation_json #>> '{request,endpoint}') as endpoint,
       (mr.request_observation_json #>> '{account,name}') as provider_account_name_snapshot,
       (mr.request_observation_json #>> '{account,email}') as provider_account_email_snapshot,
       (mr.request_observation_json #>> '{account,authenticationKind}') as provider_account_authentication_kind_snapshot,
       (mr.request_observation_json #>> '{routing,scope}') as routing_scope,
       array(select jsonb_array_elements_text(mr.request_observation_json #> '{routing,groupRefs}')) as routing_group_refs,
       (mr.request_observation_json #> '{routing,groupNamesSnapshot}') as routing_group_names_snapshot,
       (mr.request_observation_json #>> '{error,providerErrorCode}') as provider_error_code,
       (mr.request_observation_json #>> '{error,message}') as error_message,
       (mr.request_observation_json #>> '{error,retryAfterMs}')::bigint as retry_after_ms,
       (mr.request_observation_json #>> '{request,clientIp}')::inet as client_ip,
       (mr.request_observation_json #>> '{request,userAgent}') as user_agent,
       (mr.request_observation_json #>> '{request,reasoningEffort}') as reasoning_effort,
       (mr.request_observation_json #>> '{request,reasoningPreset}') as reasoning_preset,
       (mr.request_observation_json #>> '{request,subagentKind}') as subagent_kind,
       (mr.request_observation_json #>> '{request,compact}')::boolean as compact,
       measured.transport_decision_wait_ms as transport_decision_wait_ms,
       measured.connect_ms as connect_ms,
       measured.headers_ms as headers_ms,
       measured.first_event_ms as first_event_ms,
       measured.first_reasoning_ms as first_reasoning_ms,
       measured.first_text_ms as first_text_ms,
       measured.first_token_ms as first_token_ms,
       measured.latency_ms as latency_ms,
       measured.provider_processing_ms as provider_processing_ms,
       measured.admission_decision_ms as admission_decision_ms,
       measured.account_selection_wait_ms as account_selection_wait_ms,
       measured.capacity_used_slots as capacity_used_slots,
       measured.capacity_total_slots as capacity_total_slots,
       (mr.request_observation_json #>> '{transport,httpVersion}') as http_version,
       (mr.request_observation_json #>> '{transport,websocketPool}') as websocket_pool,
       (mr.request_observation_json #>> '{transport,connection,id}') as upstream_connection_id,
       (mr.request_observation_json #>> '{transport,connection,exitReason}') as upstream_connection_exit_reason,
       (mr.request_observation_json #>> '{transport,connection,ageMs}')::bigint as upstream_connection_age_ms,
       (mr.request_observation_json #>> '{transport,connection,idleMs}')::bigint as upstream_connection_idle_ms,
       (mr.request_observation_json #>> '{response,responseModel}') as upstream_response_model,
       (mr.request_observation_json #>> '{continuation,previousResponseIdHash}') as continuation_previous_response_id_hash,
       (mr.request_observation_json #>> '{continuation,unavailableReason}') as continuation_unavailable_reason,
       (mr.request_observation_json #>> '{recovery,retryDelayMs}')::bigint as recovery_retry_delay_ms,
       (mr.request_observation_json #>> '{recovery,totalLatencyMs}')::bigint as recovery_total_latency_ms,
       (mr.request_observation_json #>> '{timings,upstream,apiOverheadMs}')::double precision as upstream_api_overhead_ms,
       (mr.request_observation_json #>> '{timings,upstream,engineMs}')::double precision as upstream_engine_ms,
       (mr.request_observation_json #>> '{timings,upstream,engineIapiTtftMs}')::double precision as upstream_engine_iapi_ttft_ms,
       (mr.request_observation_json #>> '{timings,upstream,engineServiceTtftMs}')::double precision as upstream_engine_service_ttft_ms,
       (mr.request_observation_json #>> '{timings,upstream,engineIapiTbtMs}')::double precision as upstream_engine_iapi_tbt_ms,
       (mr.request_observation_json #>> '{timings,upstream,engineServiceTbtMs}')::double precision as upstream_engine_service_tbt_ms,
       measured.upstream_response_ms as upstream_response_ms,
       mr.request_observation_json #>> '{request,clientApiKeyNameSnapshot}' as client_api_key_name_snapshot,
       mr.request_observation_json #>> '{account,planTypeSnapshot}' as provider_account_plan_type_snapshot,
       mr.request_observation_json #>> '{account,notesSnapshot}' as provider_account_notes_snapshot
from model_requests mr
left join model_request_leases lease on lease.request_id = mr.id
cross join lateral (
  select (mr.request_observation_json #>> '{timings,local,transportDecisionWaitMs}')::bigint as transport_decision_wait_ms,
         (mr.request_observation_json #>> '{timings,local,connectMs}')::bigint as connect_ms,
         (mr.request_observation_json #>> '{timings,local,headersMs}')::bigint as headers_ms,
         (mr.request_observation_json #>> '{timings,local,firstEventMs}')::bigint as first_event_ms,
         (mr.request_observation_json #>> '{timings,local,firstReasoningMs}')::bigint as first_reasoning_ms,
         (mr.request_observation_json #>> '{timings,local,firstTextMs}')::bigint as first_text_ms,
         (mr.request_observation_json #>> '{timings,local,firstTokenMs}')::bigint as first_token_ms,
         (mr.request_observation_json #>> '{timings,local,latencyMs}')::bigint as latency_ms,
         (mr.request_observation_json #>> '{timings,upstream,processingMs}')::bigint as provider_processing_ms,
         (mr.request_observation_json #>> '{scheduling,admissionDecisionMs}')::bigint as admission_decision_ms,
         (mr.request_observation_json #>> '{scheduling,accountSelectionWaitMs}')::bigint as account_selection_wait_ms,
         (mr.request_observation_json #>> '{scheduling,capacityUsedSlots}')::bigint as capacity_used_slots,
         (mr.request_observation_json #>> '{scheduling,capacityTotalSlots}')::bigint as capacity_total_slots,
         (mr.request_observation_json #>> '{timings,upstream,responseMs}')::bigint as upstream_response_ms
) measured;

-- 删除 Key 时按稳定身份定位费用事件，避免扫描其他 Key 的完整账本
create index client_key_charge_events_client_api_key_id
    on client_key_charge_events (client_api_key_id);

-- 请求观测的可重建窄事实、持久化工作队列与分层统计
create sequence request_analytics_change_version;
create table request_analytics_facts as select
    mr.id, mr.started_at, mr.completed_at, mr.client_api_key_ref, mr.provider_account_ref, mr.routing_group_refs, mr.operation, mr.provider_kind, mr.requested_model_id, mr.upstream_model_id, mr.outcome, mr.client_status_code, mr.upstream_status_code, mr.client_transport, mr.upstream_transport, mr.attempt_count, mr.client_response_id, mr.upstream_response_id, mr.upstream_request_id, mr.recovered_at, mr.downstream_committed_at, mr.request_kind, mr.image_generation_requested, mr.image_generation_succeeded, mr.error_kind, mr.input_tokens, mr.output_tokens, mr.cached_input_tokens, mr.cache_write_input_tokens, mr.reasoning_output_tokens, mr.image_input_tokens, mr.image_output_tokens, mr.total_tokens, mr.cost_source, mr.cost_currency, mr.cost_amount, mr.first_token_ms, mr.latency_ms, mr.admission_decision_ms, mr.account_selection_wait_ms, mr.upstream_response_ms, mr.capacity_used_slots, mr.capacity_total_slots
from model_request_observations mr with no data;
alter table request_analytics_facts add primary key (id),
    add column standard_cost_amount numeric(20,10),
    add column no_cache_cost_amount numeric(20,10),
    add column source_version bigint not null,
    add column schema_version integer not null default 1,
    add column ops_failure_count bigint not null default 0,
    add column ops_rate_limited_count bigint not null default 0,
    add column ops_auth_failure_count bigint not null default 0,
    add column ops_provider_5xx_count bigint not null default 0;
create index request_analytics_facts_started on request_analytics_facts (started_at, id);
create index request_analytics_facts_key on request_analytics_facts (client_api_key_ref, started_at, id);
create index request_analytics_facts_account on request_analytics_facts (provider_account_ref, started_at, id);
-- 分组集合未预聚合，窄事实回退仍需按集合定位，避免低命中筛选扫描全部保留历史
create index request_analytics_facts_groups on request_analytics_facts using gin (routing_group_refs);
-- 用量列表的精确总数与账号最近使用只需要交付事实，部分索引避免为判定资格读取宽行
create index model_requests_ops_completed on model_requests (completed_at,id)
where error_kind is not null and error_kind <> 'cancelled';
create index model_requests_delivered_started on model_requests (started_at,id)
where outcome='succeeded' and downstream_committed_at is not null
    and (provider_kind is distinct from 'openai' or request_kind is distinct from 'prewarm')
    and ((client_transport='websocket' and client_status_code is null) or client_status_code between 200 and 399)
    and (requested_model_id is not null or upstream_model_id is not null or image_generation_requested
         or input_tokens is not null or output_tokens is not null or cached_input_tokens is not null
         or cache_write_input_tokens is not null or reasoning_output_tokens is not null
         or image_input_tokens is not null or image_output_tokens is not null
         or total_tokens is not null or cost_amount is not null);
create index request_analytics_facts_delivered_account on request_analytics_facts (provider_account_ref,started_at desc,id desc)
where outcome='succeeded' and downstream_committed_at is not null
    and (provider_kind is distinct from 'openai' or request_kind is distinct from 'prewarm')
    and ((client_transport='websocket' and client_status_code is null) or client_status_code between 200 and 399)
    and (requested_model_id is not null or upstream_model_id is not null or image_generation_requested
         or input_tokens is not null or output_tokens is not null or cached_input_tokens is not null
         or cache_write_input_tokens is not null or reasoning_output_tokens is not null
         or image_input_tokens is not null or image_output_tokens is not null
         or total_tokens is not null or cost_amount is not null);
-- 不透明 ID 没有私有长度上限，等值 HASH 索引避免长值超过 B-tree 索引项限制
create index request_analytics_facts_client_response on request_analytics_facts using hash (client_response_id) where client_response_id is not null;
create index request_analytics_facts_upstream_response on request_analytics_facts using hash (upstream_response_id) where upstream_response_id is not null;
create index request_analytics_facts_upstream_request on request_analytics_facts using hash (upstream_request_id) where upstream_request_id is not null;
create table request_analytics_pending_requests (
    request_id text primary key,
    change_version bigint not null default nextval('request_analytics_change_version'),
    oldest_pending_at timestamptz not null default clock_timestamp(),
    previous_bucket timestamptz,
    deletion_pending boolean not null default false
);
create index request_analytics_pending_deletions on request_analytics_pending_requests (previous_bucket) where deletion_pending;
create index request_analytics_pending_requests_age on request_analytics_pending_requests (oldest_pending_at, request_id);
create table request_analytics_pending_buckets (
    bucket_start timestamptz primary key,
    change_version bigint not null default nextval('request_analytics_change_version'),
    oldest_pending_at timestamptz not null,
    build_version bigint,
    build_phase smallint not null default 0 check (build_phase between 0 and 4),
    build_started_at timestamptz,
    build_id text,
    build_collection text,
    build_dimensions text[],
    build_outcome text
);
create index request_analytics_pending_buckets_age on request_analytics_pending_buckets(oldest_pending_at,bucket_start);
create table request_analytics_15m (
    bucket_start timestamptz not null,
    collection text not null,
    dimensions text[] not null,
    outcome text not null,
    schema_version integer not null,
    histogram_version integer not null,
    counts jsonb not null,
    request_count bigint generated always as ((counts->>'request_count')::bigint) stored,
    costs jsonb not null,
    distributions jsonb not null,
    primary key (bucket_start, collection, dimensions, outcome)
);
create table request_analytics_hourly (like request_analytics_15m including all);
create index request_analytics_15m_collection on request_analytics_15m (collection,bucket_start);
create index request_analytics_hourly_collection on request_analytics_hourly (collection,bucket_start);
create index request_analytics_15m_dimension on request_analytics_15m (collection,(dimensions[1]),bucket_start);
create index request_analytics_hourly_dimension on request_analytics_hourly (collection,(dimensions[1]),bucket_start);
-- 排名只读取确定派生的计数，覆盖索引避免为高基数账号和密钥读取宽分布行
create index request_analytics_15m_rank on request_analytics_15m (collection,bucket_start) include (dimensions,outcome,schema_version,histogram_version,request_count) where collection in ('account','key');
create index request_analytics_hourly_rank on request_analytics_hourly (collection,bucket_start) include (dimensions,outcome,schema_version,histogram_version,request_count) where collection in ('account','key');
-- 高频身份排名只保存日级精确计数，不复制分布与诊断详情
create table request_analytics_daily_counts (
    bucket_start timestamptz not null,
    collection text not null check (collection in ('account','key')),
    dimension text not null,
    outcome text not null,
    schema_version integer not null,
    histogram_version integer not null,
    request_count bigint not null check (request_count >= 0),
    primary key (collection,bucket_start,dimension,outcome,schema_version,histogram_version)
);

-- 计数由已持久化小时行的 OLD/NEW 差额派生，重放同值不会重复累计
-- 未发布小时所在的日期不参与日级查询，分段重建与版本清理仍可逐批提交
create function sync_request_analytics_daily_counts() returns trigger language plpgsql as $$
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
                update request_analytics_daily_counts set request_count=request_count+delta
                where collection=NEW.collection and bucket_start=new_day and dimension=NEW.dimensions[1]
                    and outcome=NEW.outcome and schema_version=NEW.schema_version and histogram_version=NEW.histogram_version;
            end if;
        end if;
        return null;
    end if;
    if TG_OP <> 'INSERT' and OLD.collection in ('account','key') then
        update request_analytics_daily_counts set request_count=request_count-OLD.request_count
        where collection=OLD.collection and bucket_start=old_day and dimension=OLD.dimensions[1]
            and outcome=OLD.outcome and schema_version=OLD.schema_version and histogram_version=OLD.histogram_version;
        delete from request_analytics_daily_counts
        where collection=OLD.collection and bucket_start=old_day and dimension=OLD.dimensions[1]
            and outcome=OLD.outcome and schema_version=OLD.schema_version and histogram_version=OLD.histogram_version
            and request_count=0;
    end if;
    if TG_OP <> 'DELETE' and NEW.collection in ('account','key') then
        insert into request_analytics_daily_counts values(new_day,NEW.collection,NEW.dimensions[1],NEW.outcome,NEW.schema_version,NEW.histogram_version,NEW.request_count)
        on conflict(collection,bucket_start,dimension,outcome,schema_version,histogram_version)
        do update set request_count=request_analytics_daily_counts.request_count+excluded.request_count;
    end if;
    return null;
end;
$$;
create trigger request_analytics_hourly_daily_counts
    after insert or update or delete on request_analytics_hourly
    for each row execute function sync_request_analytics_daily_counts();

create table request_analytics_build_state (
    singleton boolean primary key default true check (singleton),
    schema_version integer not null default 1,
    histogram_version integer not null default 1,
    coverage_start timestamptz,
    coverage_end timestamptz,
    backfill_cursor_started_at timestamptz,
    backfill_cursor_id text,
    backfill_complete boolean not null default false,
    reset_phase smallint not null default 0 check (reset_phase between 0 and 4),
    reset_cursor jsonb,
    generation bigint not null default 0,
    updated_at timestamptz not null default clock_timestamp(),
    check (reset_phase <> 0 or reset_cursor is null)
);
insert into request_analytics_build_state (singleton) values (true);

create function enqueue_request_analytics(request_ref text, request_start timestamptz, source_deleted boolean default false)
returns void language sql as $$
    insert into request_analytics_pending_requests(request_id, previous_bucket, deletion_pending)
    select request_ref, date_bin(interval '15 minutes', coalesce(request_start,(select started_at from request_analytics_facts where id=request_ref)), timestamptz '2000-01-01 UTC'), source_deleted
    where request_ref is not null
    on conflict (request_id) do update
    set change_version = nextval('request_analytics_change_version'),
        previous_bucket = coalesce(request_analytics_pending_requests.previous_bucket, excluded.previous_bucket),
        deletion_pending = request_analytics_pending_requests.deletion_pending or excluded.deletion_pending
$$;
create function observe_request_analytics_change() returns trigger language plpgsql as $$
begin
    if TG_OP = 'DELETE' then
        perform enqueue_request_analytics(OLD.id, OLD.started_at, true);
        return OLD;
    end if;
    perform enqueue_request_analytics(NEW.id, NEW.started_at);
    return NEW;
end
$$;
create trigger request_analytics_insert after insert on model_requests
for each row execute function observe_request_analytics_change();
create trigger request_analytics_delete after delete on model_requests
for each row execute function observe_request_analytics_change();
create trigger request_analytics_update after update on model_requests
for each row when (ROW(OLD.id, OLD.started_at, OLD.completed_at, OLD.client_api_key_ref, OLD.provider_account_ref, OLD.operation, OLD.provider_kind, OLD.requested_model_id, OLD.upstream_model_id, OLD.outcome, OLD.client_status_code, OLD.upstream_status_code, OLD.client_transport, OLD.upstream_transport, OLD.attempt_count, OLD.client_response_id, OLD.upstream_response_id, OLD.upstream_request_id, OLD.recovered_at, OLD.downstream_committed_at, OLD.request_kind, OLD.image_generation_requested, OLD.image_generation_succeeded, OLD.error_kind, OLD.input_tokens, OLD.output_tokens, OLD.cached_input_tokens, OLD.cache_write_input_tokens, OLD.reasoning_output_tokens, OLD.image_input_tokens, OLD.image_output_tokens, OLD.total_tokens, OLD.cost_source, OLD.cost_currency, OLD.cost_amount) is distinct from ROW(NEW.id, NEW.started_at, NEW.completed_at, NEW.client_api_key_ref, NEW.provider_account_ref, NEW.operation, NEW.provider_kind, NEW.requested_model_id, NEW.upstream_model_id, NEW.outcome, NEW.client_status_code, NEW.upstream_status_code, NEW.client_transport, NEW.upstream_transport, NEW.attempt_count, NEW.client_response_id, NEW.upstream_response_id, NEW.upstream_request_id, NEW.recovered_at, NEW.downstream_committed_at, NEW.request_kind, NEW.image_generation_requested, NEW.image_generation_succeeded, NEW.error_kind, NEW.input_tokens, NEW.output_tokens, NEW.cached_input_tokens, NEW.cache_write_input_tokens, NEW.reasoning_output_tokens, NEW.image_input_tokens, NEW.image_output_tokens, NEW.total_tokens, NEW.cost_source, NEW.cost_currency, NEW.cost_amount)
    or OLD.billing_snapshot_json is distinct from NEW.billing_snapshot_json
    or OLD.service_tier is distinct from NEW.service_tier
    or OLD.request_observation_json->'timings' is distinct from NEW.request_observation_json->'timings'
    or OLD.request_observation_json->'scheduling' is distinct from NEW.request_observation_json->'scheduling'
    or OLD.request_observation_json->'routing' is distinct from NEW.request_observation_json->'routing')
execute function observe_request_analytics_change();
create function observe_ops_analytics_change() returns trigger language plpgsql as $$
begin
    if TG_OP <> 'INSERT' then
        perform enqueue_request_analytics(OLD.model_request_id, null, TG_OP = 'DELETE');
    end if;
    if TG_OP <> 'DELETE' then
        perform enqueue_request_analytics(NEW.model_request_id, null);
        return NEW;
    end if;
    return OLD;
end
$$;
create trigger request_analytics_ops_insert after insert on ops_events
for each row execute function observe_ops_analytics_change();
create trigger request_analytics_ops_delete after delete on ops_events
for each row execute function observe_ops_analytics_change();
create trigger request_analytics_ops_update after update on ops_events
for each row when (ROW(OLD.model_request_id, OLD.failure_kind, OLD.status_code, OLD.attempt_index)
    is distinct from ROW(NEW.model_request_id, NEW.failure_kind, NEW.status_code, NEW.attempt_index))
execute function observe_ops_analytics_change();

-- 删除在源事务内使缓存失效，语句级触发避免批量清理争用全局行
-- 空清理不更新统计代次，避免与启动回填的快照事务发生无效写冲突
create function invalidate_deleted_request_analytics() returns trigger language plpgsql as $$
begin
    if exists (select 1 from deleted_requests) then
        update request_analytics_build_state set generation = generation + 1 where singleton;
    end if;
    return null;
end
$$;

create trigger request_analytics_delete_invalidation after delete on model_requests
referencing old table as deleted_requests
for each statement execute function invalidate_deleted_request_analytics();
create function invalidate_deleted_ops_analytics() returns trigger language plpgsql as $$
begin
    -- 父请求删除已经负责失效；独立删除仍有关联源请求的 Ops 才需要另行失效
    if exists (select 1 from deleted_ops join model_requests mr on mr.id = deleted_ops.model_request_id) then
        update request_analytics_build_state set generation = generation + 1 where singleton;
    end if;
    return null;
end
$$;
create trigger request_analytics_ops_delete_invalidation after delete on ops_events
referencing old table as deleted_ops
for each statement execute function invalidate_deleted_ops_analytics();
