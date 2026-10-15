//! 跟踪迁移目录，确保新增 SQL 文件会重新生成内嵌迁移

fn main() {
    println!("cargo:rerun-if-changed=../../migrations");
}
