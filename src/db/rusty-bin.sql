CREATE TABLE user(
    user_id INT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL
);

CREATE TABLE post(
    post_id BLOB NOT NULL PRIMARY KEY,
    user_id INT REFERENCES user(user_id),
    created_date BIGINT NOT NULL,
    expires_date BIGINT,
    language TEXT NOT NULL,
    contents BLOB NOT NULL,
    deletion_token BLOB NOT NULL
) WITHOUT ROWID;
