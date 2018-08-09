use db::schema::*;

/*
 * `category` means:
 *
 * 1 => Gliders/Motorgliders
 * 2 => Planes
 * 3 => Ultralights
 * 4 => Helicoters
 * 5 => Drones/UAV
 * 6 => Others
*/
#[derive(Queryable, Insertable)]
#[table_name="ogn_devices"]
pub struct OGNDevice {
    pub ogn_id: String,
    pub model: Option<String>,
    pub category: i16,
    pub registration: Option<String>,
    pub callsign: Option<String>,
}
