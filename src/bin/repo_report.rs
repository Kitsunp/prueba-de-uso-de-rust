use std::env;
use std::error::Error;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use tabled::settings::Style;
use tabled::{Table, Tabled};
use tokei::{Config, Languages};

#[derive(Tabled)]
struct LineRow {
    language: String,
    files: usize,
    code: usize,
    comments: usize,
    blanks: usize,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse()?;
    let root = env::current_dir()?;
    let report = build_report(&root)?;

    match args.output {
        Some(path) => {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&path, report)?;
            println!("Reporte de líneas generado en {}", path.display());
        }
        None => {
            print!("{report}");
        }
    }

    Ok(())
}

fn build_report(root: &Path) -> Result<String, Box<dyn Error>> {
    let config = Config::default();

    let mut languages = Languages::new();
    let ignored = [".git", "target"];
    languages.get_statistics(&[root], &ignored, &config);

    let mut rows: Vec<LineRow> = languages
        .iter()
        .map(|(language_type, language)| LineRow {
            language: language_type.to_string(),
            files: language.reports.len(),
            code: language.code,
            comments: language.comments,
            blanks: language.blanks,
        })
        .filter(|row| row.code + row.comments + row.blanks > 0)
        .collect();

    rows.sort_by(|left, right| right.code.cmp(&left.code));

    let total = languages.total();
    let total_files: usize = languages.iter().map(|(_, lang)| lang.reports.len()).sum();

    let table = Table::new(rows).with(Style::markdown()).to_string();

    let mut report = String::new();
    writeln!(&mut report, "# Reporte de líneas del repositorio")?;
    writeln!(&mut report, "")?;
    writeln!(&mut report, "Ruta analizada: `{}`", root.display())?;
    writeln!(&mut report, "")?;
    writeln!(&mut report, "## Desglose por lenguaje")?;
    writeln!(&mut report, "")?;
    writeln!(&mut report, "{table}")?;
    writeln!(&mut report, "")?;
    writeln!(&mut report, "## Totales")?;
    writeln!(&mut report, "")?;
    writeln!(&mut report, "- Archivos: {total_files}")?;
    writeln!(&mut report, "- Código: {}", total.code)?;
    writeln!(&mut report, "- Comentarios: {}", total.comments)?;
    writeln!(&mut report, "- Blancos: {}", total.blanks)?;

    Ok(report)
}

struct Args {
    output: Option<PathBuf>,
}

impl Args {
    fn parse() -> Result<Self, Box<dyn Error>> {
        let mut output = None;
        let mut args = env::args().skip(1);

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "-o" | "--output" => {
                    let path = args
                        .next()
                        .ok_or("Falta el argumento de ruta para --output")?;
                    output = Some(PathBuf::from(path));
                }
                "-h" | "--help" => {
                    print_help();
                    std::process::exit(0);
                }
                _ => {
                    return Err(format!("Argumento desconocido: {arg}").into());
                }
            }
        }

        Ok(Self { output })
    }
}

fn print_help() {
    println!(
        "Uso: cargo run --bin repo_report -- [--output <ruta>]\n\
\n\
Opciones:\n\
  -o, --output <ruta>  Escribe el reporte en una ruta específica\n\
  -h, --help           Muestra esta ayuda\n"
    );
}
