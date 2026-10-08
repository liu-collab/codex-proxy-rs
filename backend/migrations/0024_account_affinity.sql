-- 账号亲和模式与会话亲和时限；同一大版本内已冻结的 0023 不回改，改用本迁移补偿。
-- 上游把这几列并入其 0023 并另开 0024，本仓库 0023 已发布并落库，因此合并到本文件。
alter table runtime_settings
  add column openai_account_affinity text not null default 'relaxed'
    check (openai_account_affinity in ('relaxed', 'strict')),
  add column max_account_rotations bigint not null default 3
    check (max_account_rotations between 0 and 31),
  add column openai_session_affinity_ttl_hours bigint not null default 24
    check (openai_session_affinity_ttl_hours between 1 and 720);

-- 会话主账号优先作为独立模式，保留已有亲和配置
alter table runtime_settings
  drop constraint runtime_settings_openai_account_affinity_check,
  add constraint runtime_settings_openai_account_affinity_check
    check (openai_account_affinity in ('relaxed', 'preferred', 'strict')),
  alter column openai_account_affinity set default 'strict';

-- 尚未发布过配置的初始记录采用严格模式，已经保存的设置保留原选项
update runtime_settings set openai_account_affinity = 'strict' where config_revision = 1;
