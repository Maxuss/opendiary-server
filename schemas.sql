create table users
(
    uuid          uuid                     NOT NULL
        PRIMARY KEY,
    username      text                     NOT NULL
        UNIQUE,
    name          text                     NOT NULL,
    surname       text                     NOT NULL,
    patronymic    text,
    email         text                     NOT NULL,
    password_hash text                     NOT NULL,
    created_at    timestamp WITH TIME ZONE NOT NULL
);

create table user_sessions
(
    ssid       text                     NOT NULL
        PRIMARY KEY,
    expires_at timestamp WITH TIME ZONE NOT NULL,
    belongs_to uuid NOT NULL
);