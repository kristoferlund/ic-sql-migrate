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

        for row in rows {
            if let Ok((id, name, location, _email, rep, total, invoices, avg, first, last)) = row {
                results.push(format!(
                    "ID: {} | {} ({}) | Rep: {} | Total: ${:.2} | Invoices: {} | Avg: ${:.2} | Period: {} to {}",
                    id, name, location, rep, total, invoices, avg, first, last
                ));
            }
        }

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

        for row in rows {
            if let Ok((genre, tracks, sold, revenue, avg_price, artists)) = row {
                genre_results.push(format!(
                    "Genre: {} | Tracks: {} | Sales: {} | Revenue: ${:.2} | Avg Price: ${:.2} | Artists: {}",
                    genre, tracks, sold, revenue, avg_price, artists
                ));
            }
        }

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

        for row in rows {
            if let Ok((artist, albums, tracks, revenue, sales)) = row {
                artist_results.push(format!(
                    "Artist: {} | Albums: {} | Tracks: {} | Revenue: ${:.2} | Sales: {}",
                    artist, albums, tracks, revenue, sales
                ));
            }
        }

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

        // 1. Genre-based mega playlists
        let genres: Vec<(i32, String)> = tx
            .prepare("SELECT GenreId, Name FROM Genre")
            .unwrap()
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .filter_map(Result::ok)
            .collect();

        for (genre_id, genre_name) in &genres[..genres.len().min(15)] {
            let playlist_id = max_playlist_id + 1 + total_playlists_created;

            // Create genre playlist
            tx.execute(
                "INSERT INTO Playlist (PlaylistId, Name) VALUES (?1, ?2)",
                ic_rusqlite::params![playlist_id, format!("Ultimate {} Collection", genre_name)],
            )
            .unwrap();

            // Add all tracks from this genre plus related tracks
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
                 LIMIT 500",
                    ic_rusqlite::params![playlist_id, genre_id],
                )
                .unwrap();

            total_tracks_added += tracks_inserted;
            total_playlists_created += 1;
        }

        // 2. Year-based playlists with complex filtering
        for year in 2009..2014 {
            let playlist_id = max_playlist_id + 1 + total_playlists_created;

            tx.execute(
                "INSERT INTO Playlist (PlaylistId, Name) VALUES (?1, ?2)",
                ic_rusqlite::params![playlist_id, format!("Best of {} Retrospective", year)],
            )
            .unwrap();

            // Add tracks that were popular in that year
            let tracks_inserted = tx
                .execute(
                    "INSERT OR IGNORE INTO PlaylistTrack (PlaylistId, TrackId)
                 SELECT ?1, il.TrackId
                 FROM InvoiceLine il
                 JOIN Invoice i ON il.InvoiceId = i.InvoiceId
                 WHERE strftime('%Y', i.InvoiceDate) = ?2
                 GROUP BY il.TrackId
                 HAVING COUNT(*) > 2
                 ORDER BY COUNT(*) DESC
                 LIMIT 300",
                    ic_rusqlite::params![playlist_id, year.to_string()],
                )
                .unwrap();

            total_tracks_added += tracks_inserted;
            total_playlists_created += 1;
        }

        // 3. Create collaborative playlists (simulating user interactions)
        let active_customers: Vec<i32> = tx
            .prepare(
                "SELECT DISTINCT CustomerId
                 FROM Invoice
                 WHERE InvoiceDate > datetime('now', '-180 days')
                 LIMIT 30",
            )
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(Result::ok)
            .collect();

        for (idx, customer_id) in active_customers.iter().enumerate() {
            let playlist_id = max_playlist_id + 1 + total_playlists_created;

            // Create collaborative playlist
            tx.execute(
                "INSERT INTO Playlist (PlaylistId, Name) VALUES (?1, ?2)",
                ic_rusqlite::params![playlist_id, format!("Collaborative Mix #{}", idx + 1)],
            )
            .unwrap();

            // Add tracks based on multiple customers' preferences
            let tracks_inserted = tx
                .execute(
                    "INSERT OR IGNORE INTO PlaylistTrack (PlaylistId, TrackId)
                 SELECT ?1, t.TrackId
                 FROM Track t
                 WHERE t.TrackId IN (
                     SELECT DISTINCT il.TrackId
                     FROM InvoiceLine il
                     JOIN Invoice i ON il.InvoiceId = i.InvoiceId
                     WHERE i.CustomerId IN (?2, ?3, ?4)
                     ORDER BY RANDOM()
                     LIMIT 100
                 )",
                    ic_rusqlite::params![
                        playlist_id,
                        customer_id,
                        active_customers[(idx + 1) % active_customers.len()],
                        active_customers[(idx + 2) % active_customers.len()]
                    ],
                )
                .unwrap();

            total_tracks_added += tracks_inserted;
            total_playlists_created += 1;
        }

        // 4. Create track metadata and analytics tables
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

        // Populate analytics with complex calculations
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

        // 5. Create playlist recommendations table
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

        // Generate playlist relationships based on shared tracks
        let relationships_created = tx.execute(
            "INSERT INTO PlaylistRecommendations (PlaylistId, RecommendedPlaylistId, Score, Reason)
            SELECT
                p1.PlaylistId,
                p2.PlaylistId,
                CAST(COUNT(*) AS REAL) /
                    (SELECT COUNT(*) FROM PlaylistTrack WHERE PlaylistId = p1.PlaylistId) as Score,
                'Based on ' || COUNT(*) || ' shared tracks'
            FROM Playlist p1
            JOIN PlaylistTrack pt1 ON p1.PlaylistId = pt1.PlaylistId
            JOIN PlaylistTrack pt2 ON pt1.TrackId = pt2.TrackId
            JOIN Playlist p2 ON pt2.PlaylistId = p2.PlaylistId
            WHERE p1.PlaylistId < p2.PlaylistId
            AND p1.PlaylistId > ?
            GROUP BY p1.PlaylistId, p2.PlaylistId
            HAVING COUNT(*) > 10
            ORDER BY Score DESC
            LIMIT 500",
            ic_rusqlite::params![max_playlist_id],
        )
        .unwrap();

        // 6. Update existing playlists with metadata
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

        tx.commit().unwrap();

        (
            total_playlists_created,
            total_tracks_added,
            analytics_inserted,
            relationships_created,
            metadata_updated,
        )
    });

    let end_instructions = performance_counter(0);
    let instructions_used = end_instructions - start_instructions;

    ic_cdk::println!("Test 5 completed");
    ic_cdk::println!("Instructions used: {}", instructions_used);

    format!(
        "Test 5 completed: Created {} playlists with {} track assignments, {} analytics records, {} relationships, {} metadata entries. Instructions used: {}",
        result.0, result.1, result.2, result.3, result.4, instructions_used
    )
}
