-- 账号累计消耗要跨越用量保留期继续累计，因此单独维护按账号与币种汇总的账本
-- 账本只随完成用量事实记账；清理明细属于保留策略与人工维护，不回退已发生的累计
create table provider_account_cost_totals (
    provider_account_ref text not null,
    cost_currency text not null check (cost_currency ~ '^[A-Z]{3}$'),
    cost_amount numeric(20,10) not null check (cost_amount >= 0),
    updated_at timestamptz not null default clock_timestamp(),
    primary key (provider_account_ref,cost_currency)
);
-- 账号删除后账本行继续保留，重新导入同 ID 的账号沿用历史累计，因此不建立账号外键

-- 判定口径与 gateway-store 的 completed_usage_fact_predicate 逐条一致，改动时两处同步
create function model_request_usage_fact(request model_requests) returns boolean language sql immutable as $$
    select request.outcome = 'succeeded'
        and request.downstream_committed_at is not null
        and (request.provider_kind is distinct from 'openai' or request.request_kind is distinct from 'prewarm')
        and ((request.client_transport = 'websocket' and request.client_status_code is null)
            or request.client_status_code between 200 and 399)
        and (request.requested_model_id is not null
            or request.upstream_model_id is not null
            or request.image_generation_requested
            or request.input_tokens is not null
            or request.output_tokens is not null
            or request.cached_input_tokens is not null
            or request.cache_write_input_tokens is not null
            or request.reasoning_output_tokens is not null
            or request.image_input_tokens is not null
            or request.image_output_tokens is not null
            or request.total_tokens is not null
            or request.cost_amount is not null)
$$;

-- 返回该请求应计入账号账本的金额；未归属账号或未计价时为空，不参与记账
-- 金额非空本身已成立证据链，因此其余证据列不再重复判断
create function model_request_account_billed_amount(request model_requests) returns numeric language sql immutable as $$
    select case
        when request.provider_account_ref is not null
            and request.cost_amount is not null
            and model_request_usage_fact(request)
        then request.cost_amount
    end
$$;

create function sync_provider_account_cost_totals() returns trigger language plpgsql as $$
declare
    old_amount numeric(20,10);
    new_amount numeric(20,10);
begin
    new_amount := model_request_account_billed_amount(NEW);
    if TG_OP = 'UPDATE' then
        old_amount := model_request_account_billed_amount(OLD);
        -- 完成态、账号与金额都没变时不写账本，避免重复收尾放大同一账号行的锁争用
        if old_amount is not null and old_amount = new_amount
            and OLD.provider_account_ref = NEW.provider_account_ref
            and OLD.cost_currency = NEW.cost_currency then
            return null;
        end if;
    end if;
    -- 先减记原归属再记账，换绑账号与改写金额都在同一行事务内完成
    -- 账本余额不足时不做减记，不让历史缺口或人工清理阻断请求收尾
    if old_amount is not null then
        update provider_account_cost_totals
        set cost_amount = cost_amount - old_amount, updated_at = clock_timestamp()
        where provider_account_ref = OLD.provider_account_ref
            and cost_currency = OLD.cost_currency
            and cost_amount >= old_amount;
    end if;
    if new_amount is not null then
        insert into provider_account_cost_totals(provider_account_ref,cost_currency,cost_amount)
        values (NEW.provider_account_ref,NEW.cost_currency,new_amount)
        on conflict (provider_account_ref,cost_currency) do update
        set cost_amount = provider_account_cost_totals.cost_amount + excluded.cost_amount,
            updated_at = clock_timestamp();
    end if;
    return null;
end
$$;

-- 建立触发器自身持有该表的排他锁并延续到迁移提交，记账边界由此唯一
-- 已提交的明细被回填覆盖，其余事务的写入在解锁后由触发器记账，两者不会重复累计
-- 未归属账号或未计价的请求不进入账本，同样省去逐行调用触发器函数
create trigger provider_account_cost_totals_insert after insert on model_requests
for each row when (NEW.provider_account_ref is not null and NEW.cost_amount is not null)
execute function sync_provider_account_cost_totals();
-- 只关注决定记账结果的列：金额非空即可成立证据链，其余证据列无需放宽触发条件
create trigger provider_account_cost_totals_update after update on model_requests
for each row when (ROW(OLD.provider_account_ref, OLD.outcome, OLD.downstream_committed_at, OLD.provider_kind, OLD.request_kind, OLD.client_transport, OLD.client_status_code, OLD.cost_currency, OLD.cost_amount)
    is distinct from ROW(NEW.provider_account_ref, NEW.outcome, NEW.downstream_committed_at, NEW.provider_kind, NEW.request_kind, NEW.client_transport, NEW.client_status_code, NEW.cost_currency, NEW.cost_amount))
execute function sync_provider_account_cost_totals();

-- 只回填此刻仍在保留期内的完成用量事实，已清理明细无法追溯
-- 账本从部署时刻起继续累计，回填结果只是更接近真实值，不承诺与历史完全一致
insert into provider_account_cost_totals(provider_account_ref,cost_currency,cost_amount)
select mr.provider_account_ref, mr.cost_currency, sum(model_request_account_billed_amount(mr))
from model_requests mr
where model_request_account_billed_amount(mr) is not null
group by mr.provider_account_ref, mr.cost_currency;
