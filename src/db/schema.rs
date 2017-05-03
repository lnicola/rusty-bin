table! {
    user (user_id) {
        user_id -> Integer,
        name -> Text,
    }
}

table! {
    post (post_id) {
        post_id -> Binary,
        user_id -> Integer,
        created_date -> Timestamp,
        expres_date -> Nullable<Timestamp>,
        content -> Binary,
    }
}
