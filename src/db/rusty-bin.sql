CREATE TABLE user(
    user_id INT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL
);

CREATE TABLE post(
    post_id BLOB NOT NULL PRIMARY KEY,
    user_id INT REFERENCES user(user_id),
    created_date BIGINT NOT NULL,
    expires_date BIGINT,
    content BLOB NOT NULL
) WITHOUT ROWID;
