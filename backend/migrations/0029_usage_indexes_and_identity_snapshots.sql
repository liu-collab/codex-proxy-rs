-- Key 与账号用量列表的精确计数和双向分页只扫描已交付请求索引
create index model_requests_delivered_client on model_requests (client_api_key_ref,started_at,id)
where outcome='succeeded' and downstream_committed_at is not null
    and (provider_kind is distinct from 'openai' or request_kind is distinct from 'prewarm')
    and ((client_transport='websocket' and client_status_code is null) or client_status_code between 200 and 399)
    and (requested_model_id is not null or upstream_model_id is not null or image_generation_requested
         or input_tokens is not null or output_tokens is not null or cached_input_tokens is not null
         or cache_write_input_tokens is not null or reasoning_output_tokens is not null
         or image_input_tokens is not null or image_output_tokens is not null
         or total_tokens is not null or cost_amount is not null);

create index model_requests_delivered_account on model_requests (provider_account_ref,started_at,id)
where outcome='succeeded' and downstream_committed_at is not null
    and (provider_kind is distinct from 'openai' or request_kind is distinct from 'prewarm')
    and ((client_transport='websocket' and client_status_code is null) or client_status_code between 200 and 399)
    and (requested_model_id is not null or upstream_model_id is not null or image_generation_requested
         or input_tokens is not null or output_tokens is not null or cached_input_tokens is not null
         or cache_write_input_tokens is not null or reasoning_output_tokens is not null
         or image_input_tokens is not null or image_output_tokens is not null
         or total_tokens is not null or cost_amount is not null);

-- 只补齐快照启用前的记录，已有快照和启用后采集到的未知值保持不变
-- 补写值来自此刻仍存在的配置，不代表能够还原请求发生时的真实名称或订阅
update model_requests mr
set request_observation_json = jsonb_set(
    mr.request_observation_json,
    '{request}',
    coalesce(mr.request_observation_json -> 'request', '{}'::jsonb)
        || jsonb_build_object('clientApiKeyNameSnapshot', client_key.name)
)
from client_api_keys client_key
where client_key.id = mr.client_api_key_ref
  and mr.request_observation_json #> '{request,clientApiKeyNameSnapshot}' is null
  and mr.started_at < (
      select installed_on from _sqlx_migrations where version = 28 and success
  );

update model_requests mr
set request_observation_json = jsonb_set(
    mr.request_observation_json,
    '{account}',
    coalesce(mr.request_observation_json -> 'account', '{}'::jsonb)
        || jsonb_build_object('planTypeSnapshot', account.plan_type)
)
from provider_accounts account
where account.id = mr.provider_account_ref
  and account.plan_type is not null
  and mr.request_observation_json #> '{account,planTypeSnapshot}' is null
  and mr.started_at < (
      select installed_on from _sqlx_migrations where version = 28 and success
  );

update ops_events event
set provider_account_plan_type_snapshot = account.plan_type
from provider_accounts account
where account.id = event.provider_account_ref
  and account.plan_type is not null
  and event.provider_account_plan_type_snapshot is null
  and event.created_at < (
      select installed_on from _sqlx_migrations where version = 28 and success
  );
