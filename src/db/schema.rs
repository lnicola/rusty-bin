table! {
    user (user_id) {
        user_id -> Integer,
        name -> Text,
    }
}

table! {
    post (post_id) {
        post_id -> Binary,
        user_id -> Nullable<Integer>,
        created_date -> Timestamp,
        expires_date -> Nullable<Timestamp>,
        language -> Text,
        contents -> Binary,
        deletion_token -> Binary,
    }
}
