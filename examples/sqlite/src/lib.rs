use ic_cdk::{api::performance_counter, init, post_upgrade, pre_upgrade, query, update};
use ic_rusqlite::{close_connection, with_connection, Connection};

static MIGRATIONS: &[ic_sql_migrate::Migration] = ic_sql_migrate::include!();

fn run_migrations() {
    with_connection(|mut conn| {
        let conn: &mut Connection = &mut conn;
        ic_sql_migrate::sqlite::up(conn, MIGRATIONS).unwrap();
    });
}

#[init]
fn init() {
    run_migrations();
}

#[pre_upgrade]
fn pre_upgrade() {
    close_connection();
}

#[post_upgrade]
fn post_upgrade() {
    run_migrations();
}

#[query]
fn run() -> String {
    ic_cdk::println!("Starting migration verification...");

    let migration_count = with_connection(|mut conn| {
        let conn: &mut Connection = &mut conn;
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM _migrations", [], |row| row.get(0))
            .unwrap_or(0);
        count
    });

    ic_cdk::println!("Migrations executed: {}", migration_count);

    let total_migrations = MIGRATIONS.len() as i64;
    if migration_count == total_migrations {
        ic_cdk::println!("All {} migrations have run successfully.", total_migrations);

        // Count various tables in Chinook database
        let (customers, tracks, albums, invoices) = with_connection(|mut conn| {
            let conn: &mut Connection = &mut conn;
            let customers: i64 = conn
                .query_row("SELECT COUNT(*) FROM Customer", [], |row| row.get(0))
                .unwrap_or(0);
            let tracks: i64 = conn
                .query_row("SELECT COUNT(*) FROM Track", [], |row| row.get(0))
                .unwrap_or(0);
            let albums: i64 = conn
                .query_row("SELECT COUNT(*) FROM Album", [], |row| row.get(0))
                .unwrap_or(0);
            let invoices: i64 = conn
                .query_row("SELECT COUNT(*) FROM Invoice", [], |row| row.get(0))
                .unwrap_or(0);
            (customers, tracks, albums, invoices)
        });

        ic_cdk::println!(
            "Chinook database loaded: {} customers, {} tracks, {} albums, {} invoices",
            customers,
            tracks,
            albums,
            invoices
        );
        format!(
            "Success: All {} migrations executed. Chinook database loaded with {} customers, {} tracks, {} albums, {} invoices.",
            total_migrations, customers, tracks, albums, invoices
        )
    } else {
        ic_cdk::println!(
            "Migration verification failed: {} out of {} migrations executed.",
            migration_count,
            total_migrations
        );
        format!("Error: Only {migration_count} out of {total_migrations} migrations executed.")
    }
}

#[query]
fn test1() -> String {
    use ic_cdk::api::performance_counter;

    // Record starting instruction count
    let start_instructions = performance_counter(0);

    ic_cdk::println!("Test 1: Running top customers analysis...");

    let result = with_connection(|mut conn| {
        let conn: &mut Connection = &mut conn;

        // Query to find top 15 customers by total purchase amount with their support rep info
        let query = r#"
            SELECT
                c.CustomerId,
                c.FirstName || ' ' || c.LastName as CustomerName,
                c.City || ', ' || c.Country as Location,
                c.Email,
                e.FirstName || ' ' || e.LastName as SupportRep,
                ROUND(SUM(i.Total), 2) as TotalPurchased,
                COUNT(DISTINCT i.InvoiceId) as NumberOfInvoices,
                ROUND(AVG(i.Total), 2) as AvgInvoiceAmount,
                MIN(i.InvoiceDate) as FirstPurchase,
                MAX(i.InvoiceDate) as LastPurchase
            FROM Customer c
            JOIN Invoice i ON c.CustomerId = i.CustomerId
            LEFT JOIN Employee e ON c.SupportRepId = e.EmployeeId
            GROUP BY c.CustomerId
            ORDER BY TotalPurchased DESC
            LIMIT 15
        "#;

        let mut stmt = conn.prepare(query).unwrap();
        let mut results = Vec::new();

        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, i32>(0)?,    // CustomerId
                    row.get::<_, String>(1)?, // CustomerName
                    row.get::<_, String>(2)?, // Location
                    row.get::<_, String>(3)?, // Email
                    row.get::<_, String>(4)?, // SupportRep
                    row.get::<_, f64>(5)?,    // TotalPurchased
                    row.get::<_, i32>(6)?,    // NumberOfInvoices
                    row.get::<_, f64>(7)?,    // AvgInvoiceAmount
                    row.get::<_, String>(8)?, // FirstPurchase
                    row.get::<_, String>(9)?, // LastPurchase
                ))
            })
            .unwrap();

        rows.into_iter().for_each(|row| {
            if let Ok((id, name, location, _email, rep, total, invoices, avg, first, last)) = row {
                results.push(format!(
                    "ID: {} | {} ({}) | Rep: {} | Total: ${:.2} | Invoices: {} | Avg: ${:.2} | Period: {} to {}",
                    id, name, location, rep, total, invoices, avg, first, last
                ));
            }
        });

        results
    });

    // Record ending instruction count
    let end_instructions = performance_counter(0);
    let instructions_used = end_instructions - start_instructions;

    ic_cdk::println!("Top customers query completed");
    ic_cdk::println!("Instructions used: {}", instructions_used);

    let mut output = format!(
        "=== Top 15 Customers Analysis (Instructions: {}) ===\n\n",
        instructions_used
    );

    for (i, customer) in result.iter().enumerate() {
        output.push_str(&format!("{}. {}\n", i + 1, customer));
    }

    output
}

#[query]
fn test2() -> String {
    use ic_cdk::api::performance_counter;

    // Record starting instruction count
    let start_instructions = performance_counter(0);

    ic_cdk::println!("Test 2: Running genre and artist analysis...");

    let result = with_connection(|mut conn| {
        let conn: &mut Connection = &mut conn;

        // Analyze genres by popularity and revenue
        let genre_query = r#"
            SELECT
                g.Name as GenreName,
                COUNT(DISTINCT t.TrackId) as TrackCount,
                COUNT(DISTINCT il.InvoiceId) as TimesSold,
                ROUND(SUM(il.UnitPrice * il.Quantity), 2) as TotalRevenue,
                ROUND(AVG(il.UnitPrice), 2) as AvgPrice,
                COUNT(DISTINCT ar.ArtistId) as ArtistCount
            FROM Genre g
            JOIN Track t ON g.GenreId = t.GenreId
            LEFT JOIN InvoiceLine il ON t.TrackId = il.TrackId
            LEFT JOIN Album al ON t.AlbumId = al.AlbumId
            LEFT JOIN Artist ar ON al.ArtistId = ar.ArtistId
            GROUP BY g.GenreId
            HAVING TotalRevenue > 0
            ORDER BY TotalRevenue DESC
            LIMIT 10
        "#;

        let mut stmt = conn.prepare(genre_query).unwrap();
        let mut genre_results = Vec::new();

        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?, // GenreName
                    row.get::<_, i32>(1)?,    // TrackCount
                    row.get::<_, i32>(2)?,    // TimesSold
                    row.get::<_, f64>(3)?,    // TotalRevenue
                    row.get::<_, f64>(4)?,    // AvgPrice
                    row.get::<_, i32>(5)?,    // ArtistCount
                ))
            })
            .unwrap();

        rows.into_iter().for_each(|row| {
            if let Ok((genre, tracks, sold, revenue, avg_price, artists)) = row {
                genre_results.push(format!(
                    "Genre: {} | Tracks: {} | Sales: {} | Revenue: ${:.2} | Avg Price: ${:.2} | Artists: {}",
                    genre, tracks, sold, revenue, avg_price, artists
                ));
            }
        });

        // Top selling artists
        let artist_query = r#"
            SELECT
                ar.Name as ArtistName,
                COUNT(DISTINCT al.AlbumId) as AlbumCount,
                COUNT(DISTINCT t.TrackId) as TrackCount,
                ROUND(SUM(il.UnitPrice * il.Quantity), 2) as TotalRevenue,
                COUNT(DISTINCT il.InvoiceId) as SalesCount
            FROM Artist ar
            JOIN Album al ON ar.ArtistId = al.ArtistId
            JOIN Track t ON al.AlbumId = t.AlbumId
            LEFT JOIN InvoiceLine il ON t.TrackId = il.TrackId
            GROUP BY ar.ArtistId
            HAVING TotalRevenue > 0
            ORDER BY TotalRevenue DESC
            LIMIT 10
        "#;

        let mut stmt2 = conn.prepare(artist_query).unwrap();
        let mut artist_results = Vec::new();

        let rows = stmt2
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?, // ArtistName
                    row.get::<_, i32>(1)?,    // AlbumCount
                    row.get::<_, i32>(2)?,    // TrackCount
                    row.get::<_, f64>(3)?,    // TotalRevenue
                    row.get::<_, i32>(4)?,    // SalesCount
                ))
            })
            .unwrap();

        rows.into_iter().for_each(|row| {
            if let Ok((artist, albums, tracks, revenue, sales)) = row {
                artist_results.push(format!(
                    "Artist: {} | Albums: {} | Tracks: {} | Revenue: ${:.2} | Sales: {}",
                    artist, albums, tracks, revenue, sales
                ));
            }
        });

        (genre_results, artist_results)
    });

    // Record ending instruction count
    let end_instructions = performance_counter(0);
    let instructions_used = end_instructions - start_instructions;

    ic_cdk::println!("Genre analysis completed");
    ic_cdk::println!("Instructions used: {}", instructions_used);

    let (genres, artists) = result;

    let mut output = format!(
        "=== Genre & Artist Analysis (Instructions: {}) ===\n\n",
        instructions_used
    );

    output.push_str("Top 10 Genres by Revenue:\n");
    output.push_str("-".repeat(60).as_str());
    output.push_str("\n");
    for (i, genre) in genres.iter().enumerate() {
        output.push_str(&format!("{}. {}\n", i + 1, genre));
    }

    output.push_str("\nTop 10 Artists by Revenue:\n");
    output.push_str("-".repeat(60).as_str());
    output.push_str("\n");
    for (i, artist) in artists.iter().enumerate() {
        output.push_str(&format!("{}. {}\n", i + 1, artist));
    }

    output
}

#[query]
fn test3() -> String {
    use ic_cdk::api::performance_counter;

    // Record starting instruction count
    let start_instructions = performance_counter(0);

    ic_cdk::println!("Test 3: Running sales trends analysis...");

    let result = with_connection(|mut conn| {
        let conn: &mut Connection = &mut conn;

        // Sales by year and country
        let sales_query = r#"
            SELECT
                SUBSTR(i.InvoiceDate, 1, 4) as Year,
                i.BillingCountry as Country,
                COUNT(*) as InvoiceCount,
                ROUND(SUM(i.Total), 2) as TotalSales,
                ROUND(AVG(i.Total), 2) as AvgSale,
                COUNT(DISTINCT i.CustomerId) as UniqueCustomers
            FROM Invoice i
            GROUP BY Year, Country
            HAVING TotalSales > 50
            ORDER BY Year DESC, TotalSales DESC
            LIMIT 20
        "#;

        let mut stmt = conn.prepare(sales_query).unwrap();
        let mut sales_results = Vec::new();

        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?, // Year
                    row.get::<_, String>(1)?, // Country
                    row.get::<_, i32>(2)?,    // InvoiceCount
                    row.get::<_, f64>(3)?,    // TotalSales
                    row.get::<_, f64>(4)?,    // AvgSale
                    row.get::<_, i32>(5)?,    // UniqueCustomers
                ))
            })
            .unwrap();

        for row in rows {
            if let Ok((year, country, invoices, total, avg, customers)) = row {
                sales_results.push(format!(
                    "{} - {} | Invoices: {} | Total: ${:.2} | Avg: ${:.2} | Customers: {}",
                    year, country, invoices, total, avg, customers
                ));
            }
        }

        // Employee performance
        let employee_query = r#"
            SELECT
                e.FirstName || ' ' || e.LastName as EmployeeName,
                e.Title,
                COUNT(DISTINCT c.CustomerId) as CustomerCount,
                COUNT(DISTINCT i.InvoiceId) as InvoiceCount,
                ROUND(SUM(i.Total), 2) as TotalSales,
                ROUND(AVG(i.Total), 2) as AvgSale
            FROM Employee e
            JOIN Customer c ON e.EmployeeId = c.SupportRepId
            JOIN Invoice i ON c.CustomerId = i.CustomerId
            GROUP BY e.EmployeeId
            ORDER BY TotalSales DESC
        "#;

        let mut stmt2 = conn.prepare(employee_query).unwrap();
        let mut employee_results = Vec::new();

        let rows = stmt2
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?, // EmployeeName
                    row.get::<_, String>(1)?, // Title
                    row.get::<_, i32>(2)?,    // CustomerCount
                    row.get::<_, i32>(3)?,    // InvoiceCount
                    row.get::<_, f64>(4)?,    // TotalSales
                    row.get::<_, f64>(5)?,    // AvgSale
                ))
            })
            .unwrap();

        for row in rows {
            if let Ok((name, title, customers, invoices, total, avg)) = row {
                employee_results.push(format!(
                    "{} ({}) | Customers: {} | Invoices: {} | Total: ${:.2} | Avg: ${:.2}",
                    name, title, customers, invoices, total, avg
                ));
            }
        }

        (sales_results, employee_results)
    });

    // Record ending instruction count
    let end_instructions = performance_counter(0);
    let instructions_used = end_instructions - start_instructions;

    ic_cdk::println!("Sales trends analysis completed");
    ic_cdk::println!("Instructions used: {}", instructions_used);

    let (sales, employees) = result;

    let mut output = format!(
        "=== Sales Trends Analysis (Instructions: {}) ===\n\n",
        instructions_used
    );

    output.push_str("Top Sales by Year and Country:\n");
    output.push_str("-".repeat(60).as_str());
    output.push_str("\n");
    for (i, sale) in sales.iter().enumerate() {
        output.push_str(&format!("{}. {}\n", i + 1, sale));
    }

    output.push_str("\nEmployee Sales Performance:\n");
    output.push_str("-".repeat(60).as_str());
    output.push_str("\n");
    for (i, employee) in employees.iter().enumerate() {
        output.push_str(&format!("{}. {}\n", i + 1, employee));
    }

    output
}

#[update]
fn test4() -> String {
    // Record starting instruction count
    let start_instructions = performance_counter(0);

    ic_cdk::println!("Test 4: Massive bulk invoice generation with complex operations");

    let result = with_connection(|mut conn| {
        let conn: &mut Connection = &mut conn;

        // Get all customers and tracks for creating invoices
        let customers: Vec<(i32, String, Option<String>, String, Option<String>)> = conn
            .prepare("SELECT CustomerId, City, State, Country, PostalCode FROM Customer")
            .unwrap()
            .query_map([], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            })
            .unwrap()
            .filter_map(Result::ok)
            .collect();

        let tracks: Vec<(i32, f64, String, i32)> = conn
            .prepare("SELECT t.TrackId, t.UnitPrice, g.Name, t.Milliseconds FROM Track t JOIN Genre g ON t.GenreId = g.GenreId")
            .unwrap()
            .query_map([], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                ))
            })
            .unwrap()
            .filter_map(Result::ok)
            .collect();

        // Start transaction for bulk operations
        let tx = conn.transaction().unwrap();

        // Get the next invoice ID
        let max_invoice_id: i32 = tx
            .query_row(
                "SELECT COALESCE(MAX(InvoiceId), 0) FROM Invoice",
                [],
                |row| row.get(0),
            )
            .unwrap();

        let max_line_id: i32 = tx
            .query_row(
                "SELECT COALESCE(MAX(InvoiceLineId), 0) FROM InvoiceLine",
                [],
                |row| row.get(0),
            )
            .unwrap();

        let mut invoice_count = 0;
        let mut line_count = 0;
        let mut total_revenue = 0.0;
        let mut current_invoice_id = max_invoice_id + 1;
        let mut current_line_id = max_line_id + 1;

        // Create 250 new invoices with varying complexity
        for i in 0..250 {
            let customer_idx = (i * 7 + i * i) % customers.len();
            let (customer_id, city, state, country, postal) = &customers[customer_idx];

            // Generate invoice date spread across 2 years
            let days_offset = (i * 3) % 730;

            // Insert invoice
            tx.execute(
                "INSERT INTO Invoice (InvoiceId, CustomerId, InvoiceDate, BillingAddress, BillingCity, BillingState, BillingCountry, BillingPostalCode, Total)
                 VALUES (?1, ?2, datetime('now', '-' || ?3 || ' days'), ?4, ?5, ?6, ?7, ?8, 0.0)",
                ic_rusqlite::params![
                    current_invoice_id,
                    customer_id,
                    days_offset,
                    format!("{} Commerce Blvd Suite {}", 1000 + i, i % 200),
                    city,
                    state,
                    country,
                    postal
                ],
            )
            .unwrap();

            // Variable line items (5-25 per invoice for high complexity)
            let line_items_count = 5 + ((i * 13) % 21);
            let mut invoice_total = 0.0;

            for j in 0..line_items_count {
                // Select tracks with pattern to ensure variety
                let track_idx = ((i * 31 + j * 17) + (j * j)) % tracks.len();
                let (track_id, unit_price, _genre, _duration) = &tracks[track_idx];

                // Variable quantity (1-5) with bias towards smaller quantities
                let quantity = 1 + ((j + i) % 5);

                // Apply bulk discounts for larger quantities
                let discount_rate = if quantity > 3 { 0.9 } else { 1.0 };
                let adjusted_price = unit_price * discount_rate;

                tx.execute(
                    "INSERT INTO InvoiceLine (InvoiceLineId, InvoiceId, TrackId, UnitPrice, Quantity)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    ic_rusqlite::params![
                        current_line_id,
                        current_invoice_id,
                        track_id,
                        adjusted_price,
                        quantity
                    ],
                )
                .unwrap();

                invoice_total += adjusted_price * quantity as f64;
                current_line_id += 1;
                line_count += 1;
            }

            // Update invoice total with tax calculation (varies by country)
            let tax_rate = match country.as_str() {
                "USA" => 1.08,
                "Canada" => 1.13,
                "Brazil" => 1.17,
                _ => 1.10,
            };
            let final_total = invoice_total * tax_rate;

            tx.execute(
                "UPDATE Invoice SET Total = ?1 WHERE InvoiceId = ?2",
                ic_rusqlite::params![final_total, current_invoice_id],
            )
            .unwrap();

            total_revenue += final_total;
            current_invoice_id += 1;
            invoice_count += 1;
        }

        // Perform additional bulk updates to stress the database

        // Update customer statistics in a temporary table
        tx.execute(
            "CREATE TEMP TABLE IF NOT EXISTS CustomerStats (
                CustomerId INTEGER PRIMARY KEY,
                RecentPurchases INTEGER,
                TotalSpent REAL
            )",
            [],
        )
        .unwrap();

        tx.execute(
            "INSERT OR REPLACE INTO CustomerStats (CustomerId, RecentPurchases, TotalSpent)
            SELECT
                c.CustomerId,
                COUNT(DISTINCT i.InvoiceId) as RecentPurchases,
                COALESCE(SUM(i.Total), 0) as TotalSpent
            FROM Customer c
            LEFT JOIN Invoice i ON c.CustomerId = i.CustomerId
            WHERE i.InvoiceDate > datetime('now', '-90 days')
            GROUP BY c.CustomerId",
            [],
        )
        .unwrap();

        // Create audit log entries for new invoices
        tx.execute(
            "CREATE TABLE IF NOT EXISTS InvoiceAudit (
                AuditId INTEGER PRIMARY KEY AUTOINCREMENT,
                InvoiceId INTEGER,
                Action TEXT,
                Timestamp TEXT,
                Details TEXT
            )",
            [],
        )
        .unwrap();

        let audit_count = tx
            .execute(
                "INSERT INTO InvoiceAudit (InvoiceId, Action, Timestamp, Details)
            SELECT
                InvoiceId,
                'BULK_CREATE',
                datetime('now'),
                'Bulk invoice creation batch ' || InvoiceId
            FROM Invoice
            WHERE InvoiceId > ?",
                [max_invoice_id],
            )
            .unwrap();

        tx.commit().unwrap();
        (invoice_count, line_count, total_revenue, audit_count)
    });

    let end_instructions = performance_counter(0);
    let instructions_used = end_instructions - start_instructions;

    ic_cdk::println!("Test 4 completed");
    ic_cdk::println!("Instructions used: {}", instructions_used);

    format!(
        "Test 4 completed: Created {} invoices with {} line items (${:.2} total revenue), {} audit records. Instructions used: {}",
        result.0, result.1, result.2, result.3, instructions_used
    )
}

#[update]
fn test5() -> String {
    // Record starting instruction count
    let start_instructions = performance_counter(0);

    ic_cdk::println!("Test 5: Massive playlist generation and complex track analysis");

    let result = with_connection(|mut conn| {
        let conn: &mut Connection = &mut conn;

        // Start a large transaction
        let tx = conn.transaction().unwrap();

        // Get max playlist ID
        let max_playlist_id: i32 = tx
            .query_row(
                "SELECT COALESCE(MAX(PlaylistId), 0) FROM Playlist",
                [],
                |row| row.get(0),
            )
            .unwrap();

        let mut total_playlists_created = 0;
        let mut total_tracks_added = 0;

        // Create multiple themed playlists based on complex criteria
        // This generates diverse playlist collections to test database performance

        // 1. Genre-based playlists - create collections for each music genre
        let genres: Vec<(i32, String)> = tx
            .prepare("SELECT GenreId, Name FROM Genre")
            .unwrap()
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .filter_map(Result::ok)
            .collect();

        // Create multiple variations per genre to ensure diverse playlist collections
        for (genre_id, genre_name) in &genres {
            for variation in 0..25 {
                for sub_variation in 0..4 {
                    let playlist_id = max_playlist_id + 1 + total_playlists_created;

                    // Create genre playlist with unique naming
                    tx.execute(
                        "INSERT INTO Playlist (PlaylistId, Name) VALUES (?1, ?2)",
                        ic_rusqlite::params![playlist_id, format!("Ultimate {} Collection v{}.{}-{}", genre_name, variation + 1, sub_variation + 1, playlist_id)],
                    )
                    .unwrap();

                    // Add tracks to playlist using genre-based selection criteria
                    let tracks_inserted = tx
                        .execute(
                            "INSERT OR IGNORE INTO PlaylistTrack (PlaylistId, TrackId)
                         SELECT ?1, t.TrackId
                         FROM Track t
                         WHERE (t.GenreId = ?2
                         OR t.AlbumId IN (
                             SELECT DISTINCT t2.AlbumId
                             FROM Track t2
                             WHERE t2.GenreId = ?2
                         ))
                         AND t.TrackId % 13 = ?3  -- Use modulo for varied track selection
                         AND t.Milliseconds > ?4
                         LIMIT 8000",
                            ic_rusqlite::params![playlist_id, genre_id, variation, sub_variation * 50000],
                        )
                        .unwrap();

                    total_tracks_added += tracks_inserted;
                    total_playlists_created += 1;
                }
            }
        }

        // 2. Year-based playlists - create collections based on purchase history by year
        for year in 1985..2025 {
            for quarter in 0..4 {
                for month_variation in 0..7 {
                    let playlist_id = max_playlist_id + 1 + total_playlists_created;

                    tx.execute(
                        "INSERT INTO Playlist (PlaylistId, Name) VALUES (?1, ?2)",
                        ic_rusqlite::params![playlist_id, format!("Best of {} Q{} M{} Retrospective - {}", year, quarter + 1, month_variation + 1, playlist_id)],
                    )
                    .unwrap();

                    // Add tracks based on purchase history for the specific year and quarter
                    let tracks_inserted = tx
                        .execute(
                            "INSERT OR IGNORE INTO PlaylistTrack (PlaylistId, TrackId)
                         SELECT ?1, il.TrackId
                         FROM InvoiceLine il
                         JOIN Invoice i ON il.InvoiceId = i.InvoiceId
                         JOIN Track t ON il.TrackId = t.TrackId
                         WHERE strftime('%Y', i.InvoiceDate) = ?2
                         AND CAST(strftime('%m', i.InvoiceDate) AS INTEGER) BETWEEN (?3 * 3) + 1 AND (?3 + 1) * 3
                         AND t.GenreId % 5 = ?4
                         GROUP BY il.TrackId
                         HAVING COUNT(*) > ?5
                         ORDER BY COUNT(*) DESC, RANDOM()
                         LIMIT 5500",
                            ic_rusqlite::params![playlist_id, year.to_string(), quarter, month_variation, month_variation + 1],
                        )
                        .unwrap();

                    total_tracks_added += tracks_inserted;
                    total_playlists_created += 1;
                }
            }
        }

        // 3. Create collaborative playlists based on customer purchase patterns
        let active_customers: Vec<i32> = tx
            .prepare(
                "SELECT DISTINCT CustomerId
                 FROM Invoice
                 WHERE InvoiceDate > datetime('now', '-1095 days')
                 LIMIT 900",
            )
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(Result::ok)
            .collect();

        for (idx, customer_id) in active_customers.iter().enumerate() {
            for collab_type in 0..9 {
                for collab_subtype in 0..7 {
                    let playlist_id = max_playlist_id + 1 + total_playlists_created;

                    // Create collaborative playlist
                    tx.execute(
                        "INSERT INTO Playlist (PlaylistId, Name) VALUES (?1, ?2)",
                        ic_rusqlite::params![playlist_id, format!("Collaborative Mix #{}-{}.{}-{}", idx + 1, collab_type + 1, collab_subtype + 1, playlist_id)],
                    )
                    .unwrap();

                    // Add tracks based on collaborative customer preferences
                    let tracks_inserted = tx
                        .execute(
                            "INSERT OR IGNORE INTO PlaylistTrack (PlaylistId, TrackId)
                         SELECT ?1, t.TrackId
                         FROM Track t
                         WHERE t.TrackId IN (
                             SELECT DISTINCT il.TrackId
                             FROM InvoiceLine il
                             JOIN Invoice i ON il.InvoiceId = i.InvoiceId
                             JOIN Track t2 ON il.TrackId = t2.TrackId
                             WHERE i.CustomerId IN (?2, ?3, ?4, ?5, ?6, ?7, ?8)
                             AND il.Quantity > ?9
                             AND t2.Milliseconds > ?10
                             ORDER BY RANDOM() * il.UnitPrice * il.Quantity DESC
                             LIMIT 3500
                         )",
                            ic_rusqlite::params![
                                playlist_id,
                                customer_id,
                                active_customers[(idx + 1) % active_customers.len()],
                                active_customers[(idx + 2) % active_customers.len()],
                                active_customers[(idx + 3) % active_customers.len()],
                                active_customers[(idx + 4) % active_customers.len()],
                                active_customers[(idx + 5) % active_customers.len()],
                                active_customers[(idx + 6) % active_customers.len()],
                                collab_type + 1,
                                collab_subtype * 20000
                            ],
                        )
                        .unwrap();

                    total_tracks_added += tracks_inserted;
                    total_playlists_created += 1;
                }
            }
        }

        // 4. Create track analytics tables for performance metrics
        tx.execute(
            "CREATE TABLE IF NOT EXISTS TrackAnalytics (
                TrackId INTEGER PRIMARY KEY,
                PlayCount INTEGER DEFAULT 0,
                SkipCount INTEGER DEFAULT 0,
                Rating REAL DEFAULT 0,
                LastPlayed TEXT,
                Popularity INTEGER DEFAULT 0,
                FOREIGN KEY (TrackId) REFERENCES Track(TrackId)
            )",
            [],
        )
        .unwrap();

        // Populate basic track analytics from purchase history
        // Calculates play counts, skip rates, and ratings based on sales data
        let analytics_inserted = tx.execute(
            "INSERT OR REPLACE INTO TrackAnalytics (TrackId, PlayCount, SkipCount, Rating, LastPlayed, Popularity)
            SELECT
                t.TrackId,
                COALESCE(COUNT(DISTINCT il.InvoiceId), 0) * (1 + ABS(RANDOM() % 10)) as PlayCount,
                ABS(RANDOM() % 100) as SkipCount,
                3.0 + (RANDOM() % 20) / 10.0 as Rating,
                datetime('now', '-' || ABS(RANDOM() % 30) || ' days') as LastPlayed,
                COALESCE(COUNT(DISTINCT il.InvoiceId), 0) * 100 /
                    (1 + julianday('now') - julianday(MIN(i.InvoiceDate))) as Popularity
            FROM Track t
            LEFT JOIN InvoiceLine il ON t.TrackId = il.TrackId
            LEFT JOIN Invoice i ON il.InvoiceId = i.InvoiceId
            GROUP BY t.TrackId
            HAVING COUNT(il.InvoiceId) > 0",
            [],
        )
        .unwrap();

        // Create detailed analytics table for customer-specific metrics
        tx.execute(
            "CREATE TABLE IF NOT EXISTS TrackAnalyticsDetailed (
                TrackId INTEGER,
                CustomerId INTEGER,
                PlayCount INTEGER,
                SkipCount INTEGER,
                Rating REAL,
                LastPlayed TEXT,
                SessionId TEXT,
                FOREIGN KEY (TrackId) REFERENCES Track(TrackId),
                FOREIGN KEY (CustomerId) REFERENCES Customer(CustomerId)
            )",
            [],
        )
        .unwrap();

        // Populate detailed analytics from customer purchase patterns
        // Multiple passes with increasing complexity to stress the database
        let detailed_analytics = tx.execute(
            "INSERT INTO TrackAnalyticsDetailed (TrackId, CustomerId, PlayCount, SkipCount, Rating, LastPlayed, SessionId)
            SELECT
                il.TrackId,
                i.CustomerId,
                COUNT(*) * (1 + ABS(RANDOM() % 5)) as PlayCount,
                ABS(RANDOM() % 20) as SkipCount,
                3.5 + (RANDOM() % 15) / 10.0 as Rating,
                i.InvoiceDate as LastPlayed,
                'session_' || i.InvoiceId || '_' || il.TrackId || '_' || ABS(RANDOM() % 1000) as SessionId
            FROM InvoiceLine il
            JOIN Invoice i ON il.InvoiceId = i.InvoiceId
            JOIN Track t ON il.TrackId = t.TrackId
            JOIN Album al ON t.AlbumId = al.AlbumId
            JOIN Artist ar ON al.ArtistId = ar.ArtistId
            WHERE il.Quantity > 0
            GROUP BY il.TrackId, i.CustomerId
            HAVING COUNT(*) >= 1",  // Relaxed from > 1 to >= 1
            [],
        )
        .unwrap();

        // Additional detailed analytics pass with different criteria
        let detailed_analytics_2 = tx.execute(
            "INSERT INTO TrackAnalyticsDetailed (TrackId, CustomerId, PlayCount, SkipCount, Rating, LastPlayed, SessionId)
            SELECT
                il.TrackId,
                i.CustomerId,
                (COUNT(*) * 2) + (il.Quantity * 3) + ABS(RANDOM() % 10) as PlayCount,
                ABS(RANDOM() % 30) + (CASE WHEN il.Quantity > 2 THEN 5 ELSE 0 END) as SkipCount,
                2.5 + (RANDOM() % 25) / 10.0 + (il.UnitPrice / 2.0) as Rating,
                datetime(i.InvoiceDate, '+' || ABS(RANDOM() % 30) || ' days') as LastPlayed,
                'detailed_session_' || i.InvoiceId || '_' || il.TrackId || '_' || il.Quantity || '_' || ABS(RANDOM() % 5000) as SessionId
            FROM InvoiceLine il
            JOIN Invoice i ON il.InvoiceId = i.InvoiceId
            JOIN Track t ON il.TrackId = t.TrackId
            JOIN Genre g ON t.GenreId = g.GenreId
            WHERE il.UnitPrice > 0.5
            AND g.Name IS NOT NULL
            GROUP BY il.TrackId, i.CustomerId, il.InvoiceLineId
            HAVING COUNT(DISTINCT il.InvoiceId) >= 1",
            [],
        )
        .unwrap();

        // Third analytics pass with multi-table joins
        let detailed_analytics_3 = tx.execute(
            "INSERT INTO TrackAnalyticsDetailed (TrackId, CustomerId, PlayCount, SkipCount, Rating, LastPlayed, SessionId)
            SELECT
                il.TrackId,
                i.CustomerId,
                COUNT(*) * il.Quantity + SUM(il.Quantity) * 2 + ABS(RANDOM() % 20) as PlayCount,
                CASE WHEN ABS(RANDOM() % 50) - (il.Quantity * 2) > 0 THEN ABS(RANDOM() % 50) - (il.Quantity * 2) ELSE 0 END as SkipCount,
                CASE WHEN 3.0 + (RANDOM() % 30) / 10.0 + (il.UnitPrice * 0.1) + (COUNT(*) * 0.05) < 5.0
                     THEN 3.0 + (RANDOM() % 30) / 10.0 + (il.UnitPrice * 0.1) + (COUNT(*) * 0.05)
                     ELSE 5.0 END as Rating,
                datetime(i.InvoiceDate, '+' || (COUNT(*) * 7) || ' days') as LastPlayed,
                'complex_session_' || i.InvoiceId || '_' || il.TrackId || '_' || COUNT(*) || '_' || SUM(il.Quantity) || '_' || ABS(RANDOM() % 10000) as SessionId
            FROM InvoiceLine il
            JOIN Invoice i ON il.InvoiceId = i.InvoiceId
            JOIN Track t ON il.TrackId = t.TrackId
            JOIN Album al ON t.AlbumId = al.AlbumId
            JOIN Artist ar ON al.ArtistId = ar.ArtistId
            JOIN Genre g ON t.GenreId = g.GenreId
            WHERE il.Quantity BETWEEN 1 AND 5
            GROUP BY il.TrackId, i.CustomerId, i.BillingCountry
            HAVING SUM(il.UnitPrice * il.Quantity) > 1.0",
            [],
        )
        .unwrap();

        // Fourth analytics pass with extended time ranges
        let detailed_analytics_4 = tx.execute(
            "INSERT INTO TrackAnalyticsDetailed (TrackId, CustomerId, PlayCount, SkipCount, Rating, LastPlayed, SessionId)
            SELECT
                il.TrackId,
                i.CustomerId,
                COUNT(*) * il.Quantity * 3 + SUM(il.Quantity) * 5 + MAX(il.Quantity) * 10 + ABS(RANDOM() % 50) as PlayCount,
                CASE WHEN ABS(RANDOM() % 100) - (il.Quantity * 3) > 0 THEN ABS(RANDOM() % 100) - (il.Quantity * 3) ELSE 0 END as SkipCount,
                CASE WHEN 2.5 + (RANDOM() % 50) / 10.0 + (il.UnitPrice * 0.2) + (COUNT(*) * 0.1) + (SUM(il.Quantity) * 0.05) < 5.0
                     THEN 2.5 + (RANDOM() % 50) / 10.0 + (il.UnitPrice * 0.2) + (COUNT(*) * 0.1) + (SUM(il.Quantity) * 0.05)
                     ELSE 5.0 END as Rating,
                datetime(i.InvoiceDate, '+' || (COUNT(*) * 10) || ' days') as LastPlayed,
                'ultra_complex_session_' || i.InvoiceId || '_' || il.TrackId || '_' || COUNT(*) || '_' || SUM(il.Quantity) || '_' || MAX(il.Quantity) || '_' || ABS(RANDOM() % 50000) as SessionId
            FROM InvoiceLine il
            JOIN Invoice i ON il.InvoiceId = i.InvoiceId
            JOIN Track t ON il.TrackId = t.TrackId
            JOIN Album al ON t.AlbumId = al.AlbumId
            JOIN Artist ar ON al.ArtistId = ar.ArtistId
            JOIN Genre g ON t.GenreId = g.GenreId
            JOIN Customer c ON i.CustomerId = c.CustomerId
            WHERE il.Quantity BETWEEN 1 AND 10
            AND t.Milliseconds > 100000  -- Additional filter
            GROUP BY il.TrackId, i.CustomerId, c.Country, g.Name  -- More grouping dimensions
            HAVING SUM(il.UnitPrice * il.Quantity) > 0.5
            AND COUNT(*) > 1",
            [],
        )
        .unwrap();

        // Fifth analytics pass with employee relationship data
        let detailed_analytics_5 = tx.execute(
            "INSERT INTO TrackAnalyticsDetailed (TrackId, CustomerId, PlayCount, SkipCount, Rating, LastPlayed, SessionId)
            SELECT
                il.TrackId,
                i.CustomerId,
                COUNT(*) * il.Quantity * 5 + SUM(il.Quantity) * 8 + MAX(il.Quantity) * 15 + MIN(il.Quantity) * 3 + ABS(RANDOM() % 100) as PlayCount,
                CASE WHEN ABS(RANDOM() % 200) - (il.Quantity * 5) > 0 THEN ABS(RANDOM() % 200) - (il.Quantity * 5) ELSE 0 END as SkipCount,
                CASE WHEN 1.0 + (RANDOM() % 80) / 10.0 + (il.UnitPrice * 0.3) + (COUNT(*) * 0.15) + (SUM(il.Quantity) * 0.08) + (t.Milliseconds / 1000000.0) < 5.0
                     THEN 1.0 + (RANDOM() % 80) / 10.0 + (il.UnitPrice * 0.3) + (COUNT(*) * 0.15) + (SUM(il.Quantity) * 0.08) + (t.Milliseconds / 1000000.0)
                     ELSE 5.0 END as Rating,
                datetime(i.InvoiceDate, '+' || (COUNT(*) * 15) || ' days') as LastPlayed,
                'extreme_session_' || i.InvoiceId || '_' || il.TrackId || '_' || COUNT(*) || '_' || SUM(il.Quantity) || '_' || MAX(il.Quantity) || '_' || MIN(il.Quantity) || '_' || ABS(RANDOM() % 100000) as SessionId
            FROM InvoiceLine il
            JOIN Invoice i ON il.InvoiceId = i.InvoiceId
            JOIN Track t ON il.TrackId = t.TrackId
            JOIN Album al ON t.AlbumId = al.AlbumId
            JOIN Artist ar ON al.ArtistId = ar.ArtistId
            JOIN Genre g ON t.GenreId = g.GenreId
            JOIN Customer c ON i.CustomerId = c.CustomerId
            JOIN Employee e ON c.SupportRepId = e.EmployeeId
            WHERE il.Quantity BETWEEN 1 AND 15
            AND t.Milliseconds BETWEEN 50000 AND 1000000
            AND ar.Name IS NOT NULL
            GROUP BY il.TrackId, i.CustomerId, c.Country, g.Name, e.EmployeeId
            HAVING SUM(il.UnitPrice * il.Quantity) > 0.1
            AND COUNT(DISTINCT i.InvoiceId) > 1",
            [],
        )
        .unwrap();

        // Sixth analytics pass with enhanced calculations
        let detailed_analytics_6 = tx.execute(
            "INSERT INTO TrackAnalyticsDetailed (TrackId, CustomerId, PlayCount, SkipCount, Rating, LastPlayed, SessionId)
            SELECT
                il.TrackId,
                i.CustomerId,
                COUNT(*) * il.Quantity * 8 + SUM(il.Quantity) * 12 + MAX(il.Quantity) * 10 + MIN(il.Quantity) * 4 + ABS(RANDOM() % 150) as PlayCount,  -- Complex calculation combining multiple aggregates
                CASE WHEN ABS(RANDOM() % 300) - (il.Quantity * 8) > 0 THEN ABS(RANDOM() % 300) - (il.Quantity * 8) ELSE 0 END as SkipCount,
                CASE WHEN 0.5 + (RANDOM() % 100) / 10.0 + (il.UnitPrice * 0.4) + (COUNT(*) * 0.2) + (SUM(il.Quantity) * 0.15) + (t.Milliseconds / 1200000.0) + (t.Bytes / 1000000.0) < 5.0
                     THEN 0.5 + (RANDOM() % 100) / 10.0 + (il.UnitPrice * 0.4) + (COUNT(*) * 0.2) + (SUM(il.Quantity) * 0.15) + (t.Milliseconds / 1200000.0) + (t.Bytes / 1000000.0)
                     ELSE 5.0 END as Rating,  -- Ensure rating stays within 0-5 range
                datetime(i.InvoiceDate, '+' || (COUNT(*) * 15) || ' days') as LastPlayed,
                'advanced_session_' || i.InvoiceId || '_' || il.TrackId || '_' || COUNT(*) || '_' || SUM(il.Quantity) || '_' || ABS(RANDOM() % 150000) as SessionId
            FROM InvoiceLine il
            JOIN Invoice i ON il.InvoiceId = i.InvoiceId
            JOIN Track t ON il.TrackId = t.TrackId
            JOIN Album al ON t.AlbumId = al.AlbumId
            JOIN Artist ar ON al.ArtistId = ar.ArtistId
            JOIN Genre g ON t.GenreId = g.GenreId
            JOIN Customer c ON i.CustomerId = c.CustomerId
            WHERE il.Quantity BETWEEN 1 AND 15
            AND t.Milliseconds BETWEEN 50000 AND 1000000
            AND ar.Name IS NOT NULL
            GROUP BY il.TrackId, i.CustomerId, c.Country, g.Name
            HAVING SUM(il.UnitPrice * il.Quantity) > 0.1
            AND COUNT(DISTINCT i.InvoiceId) > 1",
            [],
        )
        .unwrap();



        // 5. Create playlist recommendations table with multiple relationship types
        tx.execute(
            "CREATE TABLE IF NOT EXISTS PlaylistRecommendations (
                RecommendationId INTEGER PRIMARY KEY AUTOINCREMENT,
                PlaylistId INTEGER,
                RecommendedPlaylistId INTEGER,
                Score REAL,
                Reason TEXT,
                CreatedAt TEXT DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (PlaylistId) REFERENCES Playlist(PlaylistId),
                FOREIGN KEY (RecommendedPlaylistId) REFERENCES Playlist(PlaylistId)
            )",
            [],
        )
        .unwrap();

        // Generate playlist recommendations based on shared tracks
        let relationships_created = tx.execute(
            "INSERT INTO PlaylistRecommendations (PlaylistId, RecommendedPlaylistId, Score, Reason)
            SELECT
                p1.PlaylistId,
                p2.PlaylistId,
                CAST(COUNT(*) AS REAL) /
                    (SELECT COUNT(*) FROM PlaylistTrack WHERE PlaylistId = p1.PlaylistId) as Score,  -- Calculate similarity score based on shared tracks
                'Based on ' || COUNT(*) || ' shared tracks - ' || p1.PlaylistId || '_' || p2.PlaylistId
            FROM Playlist p1
            JOIN PlaylistTrack pt1 ON p1.PlaylistId = pt1.PlaylistId
            JOIN PlaylistTrack pt2 ON pt1.TrackId = pt2.TrackId
            JOIN Playlist p2 ON pt2.PlaylistId = p2.PlaylistId
            WHERE p1.PlaylistId < p2.PlaylistId
            AND p1.PlaylistId > ?
            GROUP BY p1.PlaylistId, p2.PlaylistId
            HAVING COUNT(*) > 5
            ORDER BY Score DESC
            LIMIT 30000",
            ic_rusqlite::params![max_playlist_id],
        )
        .unwrap();

        // 6. Create playlist metadata for duration and statistics
        tx.execute(
            "CREATE TABLE IF NOT EXISTS PlaylistMetadata (
                PlaylistId INTEGER PRIMARY KEY,
                Duration INTEGER,
                TrackCount INTEGER,
                LastModified TEXT,
                PlayCount INTEGER DEFAULT 0,
                FOREIGN KEY (PlaylistId) REFERENCES Playlist(PlaylistId)
            )",
            [],
        )
        .unwrap();

        let metadata_updated = tx.execute(
            "INSERT OR REPLACE INTO PlaylistMetadata (PlaylistId, Duration, TrackCount, LastModified, PlayCount)
            SELECT
                p.PlaylistId,
                COALESCE(SUM(t.Milliseconds), 0) as Duration,
                COUNT(pt.TrackId) as TrackCount,
                datetime('now') as LastModified,
                ABS(RANDOM() % 1000) as PlayCount
            FROM Playlist p
            LEFT JOIN PlaylistTrack pt ON p.PlaylistId = pt.PlaylistId
            LEFT JOIN Track t ON pt.TrackId = t.TrackId
            WHERE p.PlaylistId > ?
            GROUP BY p.PlaylistId",
            ic_rusqlite::params![max_playlist_id],
        )
        .unwrap();

        // Additional metadata processing for complexity
        tx.execute(
            "CREATE TABLE IF NOT EXISTS PlaylistStats (
                PlaylistId INTEGER PRIMARY KEY,
                AvgTrackLength INTEGER,
                GenreDiversity REAL,
                PopularityScore REAL,
                LastUpdated TEXT,
                FOREIGN KEY (PlaylistId) REFERENCES Playlist(PlaylistId)
            )",
            [],
        )
        .unwrap();

        let stats_updated = tx.execute(
            "INSERT OR REPLACE INTO PlaylistStats (PlaylistId, AvgTrackLength, GenreDiversity, PopularityScore, LastUpdated)
            SELECT
                p.PlaylistId,
                AVG(t.Milliseconds) as AvgTrackLength,
                COUNT(DISTINCT t.GenreId) * 1.0 / COUNT(pt.TrackId) as GenreDiversity,
                COUNT(pt.TrackId) * (1.0 + RANDOM() % 10) as PopularityScore,
                datetime('now') as LastUpdated
            FROM Playlist p
            LEFT JOIN PlaylistTrack pt ON p.PlaylistId = pt.PlaylistId
            LEFT JOIN Track t ON pt.TrackId = t.TrackId
            WHERE p.PlaylistId > ?
            GROUP BY p.PlaylistId
            HAVING COUNT(pt.TrackId) > 0",
            ic_rusqlite::params![max_playlist_id],
        )
        .unwrap();

        // 7. Create playlist versioning system for additional complexity
        tx.execute(
            "CREATE TABLE IF NOT EXISTS PlaylistVersions (
                VersionId INTEGER PRIMARY KEY AUTOINCREMENT,
                PlaylistId INTEGER,
                VersionNumber INTEGER,
                TrackCount INTEGER,
                CreatedAt TEXT,
                Changes TEXT,
                FOREIGN KEY (PlaylistId) REFERENCES Playlist(PlaylistId)
            )",
            [],
        )
        .unwrap();

        let versions_created = tx.execute(
            "INSERT INTO PlaylistVersions (PlaylistId, VersionNumber, TrackCount, CreatedAt, Changes)
            SELECT
                p.PlaylistId,
                1 as VersionNumber,
                COUNT(pt.TrackId) as TrackCount,
                datetime('now') as CreatedAt,
                'Initial version with ' || COUNT(pt.TrackId) || ' tracks'
            FROM Playlist p
            LEFT JOIN PlaylistTrack pt ON p.PlaylistId = pt.PlaylistId
            WHERE p.PlaylistId > ?
            GROUP BY p.PlaylistId",
            ic_rusqlite::params![max_playlist_id],
        )
        .unwrap();

        // 8. Create user interaction data for playlist engagement
        tx.execute(
            "CREATE TABLE IF NOT EXISTS UserPlaylistInteractions (
                InteractionId INTEGER PRIMARY KEY AUTOINCREMENT,
                CustomerId INTEGER,
                PlaylistId INTEGER,
                InteractionType TEXT,
                Rating INTEGER,
                Timestamp TEXT,
                FOREIGN KEY (CustomerId) REFERENCES Customer(CustomerId),
                FOREIGN KEY (PlaylistId) REFERENCES Playlist(PlaylistId)
            )",
            [],
        )
        .unwrap();

        // 9. Create playlist optimization recommendations
        tx.execute(
            "CREATE TABLE IF NOT EXISTS PlaylistOptimizations (
                OptimizationId INTEGER PRIMARY KEY AUTOINCREMENT,
                PlaylistId INTEGER,
                OptimizationType TEXT,
                Score REAL,
                Description TEXT,
                EstimatedImprovement REAL,
                CreatedAt TEXT DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (PlaylistId) REFERENCES Playlist(PlaylistId)
            )",
            [],
        )
        .unwrap();

        let optimizations_created = tx.execute(
            "INSERT INTO PlaylistOptimizations (PlaylistId, OptimizationType, Score, Description, EstimatedImprovement)
            SELECT
                p.PlaylistId,
                CASE ABS(RANDOM() % 5)
                    WHEN 0 THEN 'duplicate_removal'
                    WHEN 1 THEN 'genre_balance'
                    WHEN 2 THEN 'popularity_boost'
                    WHEN 3 THEN 'duration_optimization'
                    ELSE 'diversity_enhancement'
                END as OptimizationType,
                (COUNT(DISTINCT t.GenreId) * 1.0 / COUNT(pt.TrackId)) * (AVG(t.Milliseconds) / 100000.0) as Score,
                'Optimization analysis for playlist ' || p.PlaylistId || ' with ' || COUNT(pt.TrackId) || ' tracks' as Description,
                ABS(RANDOM() % 50) / 10.0 as EstimatedImprovement
            FROM Playlist p
            LEFT JOIN PlaylistTrack pt ON p.PlaylistId = pt.PlaylistId
            LEFT JOIN Track t ON pt.TrackId = t.TrackId
            WHERE p.PlaylistId > ?
            GROUP BY p.PlaylistId
            HAVING COUNT(pt.TrackId) > 5",
            ic_rusqlite::params![max_playlist_id],
        )
        .unwrap();

        // 10. Create track similarity matrix for recommendations
        // Compares tracks based on genre, duration, and other attributes
        tx.execute(
            "CREATE TABLE IF NOT EXISTS TrackSimilarities (
                TrackId1 INTEGER,
                TrackId2 INTEGER,
                SimilarityScore REAL,
                CommonGenres INTEGER,
                DurationDifference INTEGER,
                ArtistSimilarity REAL,
                CalculatedAt TEXT DEFAULT CURRENT_TIMESTAMP,
                PRIMARY KEY (TrackId1, TrackId2),
                FOREIGN KEY (TrackId1) REFERENCES Track(TrackId),
                FOREIGN KEY (TrackId2) REFERENCES Track(TrackId)
            )",
            [],
        )
        .unwrap();

        let similarities_created = tx.execute(
            "INSERT OR IGNORE INTO TrackSimilarities (TrackId1, TrackId2, SimilarityScore, CommonGenres, DurationDifference, ArtistSimilarity)
            SELECT
                t1.TrackId as TrackId1,
                t2.TrackId as TrackId2,
                (CASE WHEN t1.GenreId = t2.GenreId THEN 1.0 ELSE 0.5 END) *
                (1.0 - ABS(t1.Milliseconds - t2.Milliseconds) / 1000000.0) as SimilarityScore,
                CASE WHEN t1.GenreId = t2.GenreId THEN 1 ELSE 0 END as CommonGenres,
                ABS(t1.Milliseconds - t2.Milliseconds) as DurationDifference,
                0.5 as ArtistSimilarity
            FROM Track t1, Track t2
            WHERE t1.TrackId < t2.TrackId
            AND t1.TrackId % 10 = 0  -- Sample every 10th track to control computation
            AND t2.TrackId % 10 = 0
            LIMIT 12000",
            [],
        )
        .unwrap();

        // 11. Create additional user interaction data

        let interactions_created = tx.execute(
            "INSERT INTO UserPlaylistInteractions (CustomerId, PlaylistId, InteractionType, Rating, Timestamp)
            SELECT
                c.CustomerId,
                p.PlaylistId,
                CASE ABS(RANDOM() % 6)  -- Generate random interaction types
                    WHEN 0 THEN 'played'
                    WHEN 1 THEN 'liked'
                    WHEN 2 THEN 'shared'
                    WHEN 3 THEN 'saved'
                    WHEN 4 THEN 'followed'
                    ELSE 'commented'
                END as InteractionType,
                2 + ABS(RANDOM() % 4) as Rating,
                datetime('now', '-' || ABS(RANDOM() % 730) || ' days') as Timestamp
            FROM Customer c
            CROSS JOIN (
                SELECT PlaylistId FROM Playlist WHERE PlaylistId > ? LIMIT 2000
            ) p
            WHERE c.CustomerId IN (SELECT DISTINCT CustomerId FROM Invoice)
            LIMIT 220000",
            ic_rusqlite::params![max_playlist_id],
        )
        .unwrap();

        tx.commit().unwrap();

        // Return tuple with counts of all operations performed
        (
            total_playlists_created,
            total_tracks_added,
            analytics_inserted,
            relationships_created,
            metadata_updated,
            detailed_analytics + detailed_analytics_2 + detailed_analytics_3 + detailed_analytics_4 + detailed_analytics_5 + detailed_analytics_6,  // Sum all detailed analytics
            stats_updated,
            versions_created,
            interactions_created,
            optimizations_created,
            similarities_created,
        )
    });

    let end_instructions = performance_counter(0);
    let instructions_used = end_instructions - start_instructions;

    ic_cdk::println!("Test 5 completed");
    ic_cdk::println!("Instructions used: {}", instructions_used);

    format!(
        "Test 5 completed: Created {} playlists with {} track assignments, {} analytics records, {} relationships, {} metadata entries, {} detailed analytics, {} stats, {} versions, {} interactions, {} optimizations, {} similarities. Instructions used: {}",
        result.0, result.1, result.2, result.3, result.4, result.5, result.6, result.7, result.8, result.9, result.10, instructions_used
    )
}

mod benches {
    use super::*;
    use canbench_rs::bench;

    #[bench]
    fn test1_top_customers_analysis() {
        test1();
    }

    #[bench]
    fn test2_genre_and_artist_analysis() {
        test2();
    }

    #[bench]
    fn test3_sales_trends_analysis() {
        test3();
    }

    #[bench]
    fn test4_massive_bulk_invoice_generation_with_complex_operations() {
        test4();
    }

    #[bench]
    fn test5_massive_playlist_generation_and_complex_track_analysis() {
        test5();
    }
}
