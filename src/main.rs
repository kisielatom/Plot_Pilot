use csv::ReaderBuilder;
use rocket::fs::{FileServer, NamedFile};
use rocket::serde::json::Json;
use rocket::tokio::fs::File as OtherFile;
use rocket::tokio::io::AsyncReadExt;
use rocket::State;
use serde_derive::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs::File;
use std::path::{Path, PathBuf};

#[macro_use]
extern crate rocket;

#[derive(Debug, Deserialize)]
struct CsvRow {
    #[serde(rename = "date")]
    date: String,
    #[serde(rename = "flight")]
    flight: String,
    #[serde(rename = "from")]
    from: String,
    #[serde(rename = "to")]
    to: String,
    // ... other columns
}

#[get("/count_iata_from")]
fn count_iata_from(state: &State<SharedData>) -> Json<HashMap<String, usize>> {
    let data = &state.data;
    let mut iata_counts: HashMap<String, usize> = HashMap::new();

    for row in data {
        *iata_counts.entry(row.from.clone()).or_insert(0) += 1;
    }
    Json(iata_counts)
}

#[get("/count_iata_to")]
fn count_iata_to(state: &State<SharedData>) -> Json<HashMap<String, usize>> {
    let data = &state.data;
    let mut iata_counts: HashMap<String, usize> = HashMap::new();

    for row in data {
        *iata_counts.entry(row.to.clone()).or_insert(0) += 1;
    }
    Json(iata_counts)
}

#[get("/data/<file..>")]
async fn get_file(file: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new("data/").join(file)).await.ok()
}
#[derive(Serialize, Deserialize)]
struct CombinedMapping {
    location: String,
    iata: String,
    lon: f64,
    lat: f64,
    count: usize,
}

async fn load_iata_lon_lat() -> Result<HashMap<String, (f64, f64)>, std::io::Error> {
    let mut mapping = HashMap::new();

    let mut file = OtherFile::open("data/airports.json").await?;
    let mut content = String::new();
    file.read_to_string(&mut content).await?;

    let json_data: serde_json::Value = serde_json::from_str(&content)?;
    if let serde_json::Value::Array(airports) = json_data {
        for airport in airports {
            let iata = airport["iata"].as_str().unwrap_or_default();
            let lon = airport["lon"]
                .as_str()
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0);
            let lat = airport["lat"]
                .as_str()
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0);

            mapping.insert(iata.to_string(), (lon, lat));
        }
    }
    Ok(mapping)
}

fn process_location(
    iata: &str,
    lon_lat: Option<&(f64, f64)>,
    count: usize,
    added_codes: &mut HashSet<String>,
    combined_data: &mut Vec<CombinedMapping>,
    location: &str,
) {
    if let Some((lon, lat)) = lon_lat {
        if added_codes.insert(iata.to_string()) {
            let mapping = CombinedMapping {
                location: location.to_string(),
                iata: iata.to_string(),
                lon: *lon,
                lat: *lat,
                count,
            };
            combined_data.push(mapping);
        }
    }
}

#[get("/combined_data")]
async fn combined_data(state: &State<SharedData>) -> Json<Vec<CombinedMapping>> {
    let json_data = &state.data; // CSV data
    let from_counts = count_iata_from(state).into_inner(); // Counts from count_iata_from
    let to_counts = count_iata_to(state).into_inner(); // Counts from count_iata_to
    let iata_mapping = load_iata_lon_lat().await.unwrap(); // IATA to lon/lat mapping

    let mut combined_data: Vec<CombinedMapping> = Vec::new();
    let mut added_iata_codes_from: HashSet<String> = HashSet::new(); // Keep track of added IATA codes
    let mut added_iata_codes_to: HashSet<String> = HashSet::new(); // Keep track of added IATA codes

    for row in json_data {
        let from_count = from_counts.get(&row.from).cloned().unwrap_or(0);
        let to_count = to_counts.get(&row.to).cloned().unwrap_or(0);

        let from = row.from.clone();
        let to = row.to.clone();

        process_location(
            &from,
            iata_mapping.get(&from),
            from_count,
            &mut added_iata_codes_from,
            &mut combined_data,
            "From",
        );
        process_location(
            &to,
            iata_mapping.get(&to),
            to_count,
            &mut added_iata_codes_to,
            &mut combined_data,
            "To",
        );
    }

    Json(combined_data)
}

struct SharedData {
    data: Vec<CsvRow>,
}

#[derive(Debug, Deserialize, Serialize)]
struct IataCityMapping {
    iata: String,
    lat: usize,
    long: usize,
}

#[rocket::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Read CSV file
    let file_path = Path::new("data").join("Logbook-1.csv");
    let file = File::open(file_path)?;
    let mut csv_reader = ReaderBuilder::new()
        .has_headers(true)
        .delimiter(b',')
        .from_reader(file);

    let mut data = Vec::new();
    for result in csv_reader.deserialize() {
        let row: CsvRow = result?;
        data.push(row);
    }

    let shared_data = SharedData { data };

    rocket::build()
        .manage(shared_data)
        .mount("/", routes![count_iata_from, count_iata_to, combined_data])
        .mount("/", FileServer::from("static"))
        .mount("/", routes![get_file])
        .launch()
        .await?;

    Ok(())
}
