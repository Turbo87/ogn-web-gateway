use chrono::NaiveDateTime;

use db::schema::ogn_positions;

#[derive(Message, Queryable, Insertable)]
#[table_name="ogn_positions"]
pub struct OGNPosition {
    pub ogn_id: String,
    pub time: NaiveDateTime,
    pub longitude: f64,
    pub latitude: f64,
}

pub type CreateOGNPosition = OGNPosition;
