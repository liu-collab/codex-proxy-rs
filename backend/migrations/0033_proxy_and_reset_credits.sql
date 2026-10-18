alter table outbound_proxies
    add column max_websocket_connections bigint not null default 0
    check (max_websocket_connections between 0 and 4294967295);

-- 主动重置卡配置与计划随账号保存，到期索引只覆盖开启自动使用的账号
ALTER TABLE runtime_settings
    ADD COLUMN reset_credit_lead_minutes integer NOT NULL DEFAULT 10
        CHECK (reset_credit_lead_minutes BETWEEN 1 AND 1440);

ALTER TABLE provider_accounts
    ADD COLUMN reset_credit_automation_json jsonb,
    ADD COLUMN reset_credit_next_run_at timestamptz,
    ADD CONSTRAINT provider_accounts_reset_credit_automation_shape CHECK (
        (reset_credit_automation_json IS NULL AND reset_credit_next_run_at IS NULL)
        OR coalesce((
            reset_credit_automation_json IS NOT NULL AND reset_credit_next_run_at IS NOT NULL
            AND jsonb_typeof(reset_credit_automation_json) = 'object'
            AND jsonb_typeof(reset_credit_automation_json -> 'enabled') = 'boolean'
            AND jsonb_typeof(reset_credit_automation_json -> 'revision') = 'number'
            AND jsonb_typeof(reset_credit_automation_json -> 'state') = 'object'
        ), false)
    );

CREATE INDEX provider_accounts_reset_credit_due_idx
    ON provider_accounts (reset_credit_next_run_at, id)
    WHERE enabled AND reset_credit_automation_json ->> 'enabled' = 'true';

-- 自动使用重置卡的剩余周额度上限，空值表示不限制
ALTER TABLE runtime_settings
    ADD COLUMN reset_credit_max_weekly_remaining_percent integer
        CHECK (reset_credit_max_weekly_remaining_percent BETWEEN 0 AND 100);
