use ic_cdk::{
    api::{performance_counter, time},
    init, post_upgrade, pre_upgrade, query, update,
};
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

        let person_count = with_connection(|mut conn| {
            let conn: &mut Connection = &mut conn;
            let count: i64 = conn
                .query_row("SELECT COUNT(*) FROM person", [], |row| row.get(0))
                .unwrap_or(0);
            count
        });

        ic_cdk::println!("Found {} records in person table.", person_count);
        format!(
            "Success: All {total_migrations} migrations executed. {person_count} persons in database."
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
fn chinook_top_customers() -> String {
    use ic_cdk::api::performance_counter;

    // Record starting instruction count
    let start_instructions = performance_counter(0);

    ic_cdk::println!("Running Chinook top customers analysis...");

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
            if let Ok((id, name, location, email, rep, total, invoices, avg, first, last)) = row {
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
fn chinook_genre_analysis() -> String {
    use ic_cdk::api::performance_counter;

    // Record starting instruction count
    let start_instructions = performance_counter(0);

    ic_cdk::println!("Running Chinook genre and artist analysis...");

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
fn chinook_sales_trends() -> String {
    use ic_cdk::api::performance_counter;

    // Record starting instruction count
    let start_instructions = performance_counter(0);

    ic_cdk::println!("Running Chinook sales trends analysis...");

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
fn perf1() -> String {
    // Record starting instruction count and time
    let start_instructions = performance_counter(0);
    let start_time = time();

    ic_cdk::println!("Starting performance test: inserting 1000 records with ~1KB data each");

    // Generate random seed from current time
    let seed = start_time as u32;

    let result = with_connection(|mut conn| {
        let conn: &mut Connection = &mut conn;

        // Start a transaction for better performance
        let tx = conn.transaction().unwrap();

        // Prepare the insert statement
        let mut stmt = tx
            .prepare("INSERT INTO perf_test (data, random_value) VALUES (?1, ?2)")
            .unwrap();

        // Insert 1000 records
        for i in 0..1000 {
            // Generate ~1KB of random data
            let data = generate_random_data(seed + i, 1024);
            let random_value = ((seed + i) * 2654435761) % 1000000; // Simple hash for random value

            stmt.execute([
                &data as &dyn ic_rusqlite::ToSql,
                &random_value as &dyn ic_rusqlite::ToSql,
            ])
            .unwrap();
        }

        drop(stmt);
        tx.commit().unwrap();

        // Count total records in the table
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM perf_test", [], |row| row.get(0))
            .unwrap();

        count
    });

    // Record ending instruction count and time
    let end_instructions = performance_counter(0);
    let instructions_used = end_instructions - start_instructions;

    ic_cdk::println!("Performance test completed");
    ic_cdk::println!("Instructions used: {}", instructions_used);
    ic_cdk::println!("Total records in perf_test table: {}", result);

    format!(
        "Performance test completed: Inserted 1000 records. Instructions used: {instructions_used}. Total records: {result}"
    )
}

/// Generate random-looking data of specified size
fn generate_random_data(seed: u32, size: usize) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut result = String::with_capacity(size);
    let mut current = seed;

    for _ in 0..size {
        // Simple linear congruential generator
        current = ((current as u64 * 1664525 + 1013904223) % (1 << 32)) as u32;
        let char_index = (current % CHARS.len() as u32) as usize;
        result.push(CHARS[char_index] as char);
    }

    result
}
