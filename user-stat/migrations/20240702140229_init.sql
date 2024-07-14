-- Add migration script here
CREATE TYPE gender AS ENUM ('male', 'female', 'unknown');

CREATE TABLE user_stats(
    -- 邮箱地址，主键，长度为128字符，不能为空
    email varchar(128) NOT NULL PRIMARY KEY,
    -- 用户名，长度为64字符，不能为空
    name varchar(64) NOT NULL,
    -- 性别，默认值为unknown
    gender gender DEFAULT 'unknown',
    -- 创建时间，默认值为当前时间
    created_at timestamptz DEFAULT CURRENT_TIMESTAMP,
    -- 最后访问时间
    last_visited_at timestamptz,
    -- 最后观看时间
    last_watched_at timestamptz,
    -- 最近观看的视频id，用数组表示
    recent_watched int[],
    -- 查看但未开始的视频id，用数组表示
    viewed_but_not_started int[],
    -- 开始但未完成的视频id，用数组表示
    started_but_not_finished int[],
    -- 完成的视频id，用数组表示
    finished int[],
    -- 最后发送邮件通知的时间
    last_email_notification timestamptz,
    -- 最后发送应用通知的时间
    last_in_app_notification timestamptz,
    -- 最后发送短信通知的时间
    last_sms_notification timestamptz
);
