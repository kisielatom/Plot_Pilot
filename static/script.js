// Initialize the map
var map = L.map('map').setView([46.603354, 1.888334], 4);

// Add the map tiles from OpenStreetMap
L.tileLayer('https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png', {
  maxZoom: 19,
}).addTo(map);

// Fetch the JSON data using the Fetch API
fetch('/combined_data')
    .then(response => response.json())
    .then(data => {
        try {
            console.log(data);
            for (var i = 0; i < data.length; i++) {
                if (data[i].lat !== undefined && data[i].lon !== undefined) {
                    var marker = L.marker([data[i].lat, data[i].lon]).addTo(map);
                    marker.bindPopup(`${data[i].iata}<br>Times : ${data[i].count}`);
                }
            }
        } catch (e) {
            console.error('Error parsing JSON:', e);
        }
    })
    .catch(error => {
        console.error('Error fetching JSON:', error);
    });
