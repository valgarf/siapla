from functools import reduce
from pathlib import Path

base = Path("./crates/siapla/src/entity")
rev = base / "revision.rs"

content = rev.read_text(encoding="utf-8")
content = content.replace("pub id: i32,", "pub id: i64,")
content = content.replace("type ValueType = i32;", "type ValueType = i64;")
content = content.replace(
    "Self::Id => ColumnType::Integer.def(),",
    "Self::Id => ColumnType::BigInteger.def(),",
)
rev.write_text(content, encoding="utf-8")

replacements = [
    ("pub revision: i32,", "pub revision: i64,"),
    ("pub rev_created: i32,", "pub rev_created: i64,"),
    ("pub rev_deleted: Option<i32>,", "pub rev_deleted: Option<i64>,"),
    (
        "Self::Revision => ColumnType::Integer.def(),",
        "Self::Revision => ColumnType::BigInteger.def(),",
    ),
    (
        "Self::RevCreated => ColumnType::Integer.def(),",
        "Self::RevCreated => ColumnType::BigInteger.def(),",
    ),
    (
        "Self::RevDeleted => ColumnType::Integer.def().null(),",
        "Self::RevDeleted => ColumnType::BigInteger.def().null(),",
    ),
]

for path in base.glob("*.rs"):
    text = path.read_text(encoding="utf-8")
    text = reduce(lambda current, repl: current.replace(*repl), replacements, text)
    path.write_text(text, encoding="utf-8")
