use chrono::{DateTime, Utc};

use db::schema::ogn_positions;

#[derive(Queryable, Insertable)]
#[table_name="ogn_positions"]
pub struct OGNPosition {
    pub ogn_id: String,
    pub time: DateTime<Utc>,
    pub longitude: f64,
    pub latitude: f64,
    pub altitude: i32,
}
