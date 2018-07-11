use chrono::NaiveDateTime;

use db::schema::ogn_positions;

#[derive(Message, Insertable)]
#[table_name="ogn_positions"]
pub struct CreateOGNPosition {
    pub ogn_id: String,
    pub time: NaiveDateTime,
    pub longitude: f64,
    pub latitude: f64,
}
