// `sqlx::migrate!` embeds the migration files at compile time, and cargo does
// not see a newly added file on its own: without this, a build keeps the old
// list and a test database is created without the new migration.
fn main() {
    println!("cargo:rerun-if-changed=migrations");
}
