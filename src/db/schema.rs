table! {
    ogn_positions (ogn_id, time) {
        ogn_id -> Text,
        time -> Timestamp,
        longitude -> Float8,
        latitude -> Float8,
        altitude -> Int4,
    }
}

table! {
    spatial_ref_sys (srid) {
        srid -> Int4,
        auth_name -> Nullable<Varchar>,
        auth_srid -> Nullable<Int4>,
        srtext -> Nullable<Varchar>,
        proj4text -> Nullable<Varchar>,
    }
}

allow_tables_to_appear_in_same_query!(
    ogn_positions,
    spatial_ref_sys,
);
