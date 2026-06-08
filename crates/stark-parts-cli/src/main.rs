use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use stark_parts_catalog::{
    CrawlConfig, DEFAULT_CATALOG_PATH, StarkHttpClient, UpstreamCatalog, crawl_catalog,
    format_catalog_json5, parse_catalog_json5,
};
use std::fs;
use std::path::Path;
use std::process::ExitCode;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use tracing::{info, instrument};
use tracing_subscriber::EnvFilter;

/// Command line entrypoint for maintaining the committed Stark catalog.
#[derive(Debug, Parser)]
#[command(name = "stark-parts")]
#[command(about = "Maintain the Stark parts catalog")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Commands that read or write the committed catalog state.
    Catalog {
        #[command(subcommand)]
        command: CatalogCommand,
    },
}

#[derive(Clone, Copy, Debug, Subcommand)]
enum CatalogCommand {
    /// Create the initial committed catalog.
    Init,
    /// Refresh an existing committed catalog.
    Update,
}

fn main() -> ExitCode {
    init_logging();

    match run_cli(Cli::parse()) {
        Ok(message) => {
            println!("{message}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error:#}");
            ExitCode::FAILURE
        }
    }
}

fn init_logging() {
    let _ = tracing::subscriber::set_global_default(build_logging_subscriber(
        EnvFilter::from_default_env(),
    ));
}

fn build_logging_subscriber(filter: EnvFilter) -> impl tracing::Subscriber + Send + Sync + 'static {
    tracing_subscriber::fmt().with_env_filter(filter).finish()
}

fn run_cli(cli: Cli) -> Result<String> {
    let cwd = std::env::current_dir().context("failed to read current directory")?;
    preflight(&cli, &cwd)?;
    let config = CrawlConfig::us_storefront(current_rfc3339_timestamp()?);
    let client = StarkHttpClient::new(&config)?;
    run_with(cli, &cwd, &client, config)
}

fn preflight(cli: &Cli, repo_root: &Path) -> Result<()> {
    match &cli.command {
        Command::Catalog { command } => preflight_catalog_command(repo_root, *command),
    }
}

fn preflight_catalog_command(repo_root: &Path, command: CatalogCommand) -> Result<()> {
    ensure_repository_root(repo_root)?;
    let catalog_path = repo_root.join(DEFAULT_CATALOG_PATH);
    reject_catalog_path_symlinks(repo_root, &catalog_path)?;

    match command {
        CatalogCommand::Init if catalog_path.exists() => {
            bail!("catalog already exists at {}", catalog_path.display());
        }
        CatalogCommand::Update if !catalog_path.exists() => {
            bail!("catalog does not exist at {}", catalog_path.display());
        }
        CatalogCommand::Update => {
            let existing = fs::read_to_string(&catalog_path)
                .with_context(|| format!("failed to read {}", catalog_path.display()))?;
            parse_catalog_json5(&existing).with_context(|| {
                format!(
                    "existing catalog at {} is not valid committed catalog JSON5",
                    catalog_path.display()
                )
            })?;
            Ok(())
        }
        _ => Ok(()),
    }
}

#[instrument(skip(cli, client, config), fields(repo_root = %repo_root.display()))]
fn run_with(
    cli: Cli,
    repo_root: &Path,
    client: &impl UpstreamCatalog,
    config: CrawlConfig,
) -> Result<String> {
    match cli.command {
        Command::Catalog {
            command: CatalogCommand::Init,
        } => {
            info!("catalog init requested");
            write_catalog(repo_root, client, &config, WriteMode::Init)
        }
        Command::Catalog {
            command: CatalogCommand::Update,
        } => {
            info!("catalog update requested");
            write_catalog(repo_root, client, &config, WriteMode::Update)
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WriteMode {
    Init,
    Update,
}

fn write_catalog(
    repo_root: &Path,
    client: &impl UpstreamCatalog,
    config: &CrawlConfig,
    mode: WriteMode,
) -> Result<String> {
    ensure_repository_root(repo_root)?;

    let catalog_path = repo_root.join(DEFAULT_CATALOG_PATH);
    reject_catalog_path_symlinks(repo_root, &catalog_path)?;
    match mode {
        WriteMode::Init if catalog_path.exists() => {
            bail!("catalog already exists at {}", catalog_path.display());
        }
        WriteMode::Update if !catalog_path.exists() => {
            bail!("catalog does not exist at {}", catalog_path.display());
        }
        _ => {}
    }

    let existing = if catalog_path.exists() {
        let raw = fs::read_to_string(&catalog_path)
            .with_context(|| format!("failed to read {}", catalog_path.display()))?;
        let parsed = parse_catalog_json5(&raw).with_context(|| {
            format!(
                "existing catalog at {} is not valid committed catalog JSON5",
                catalog_path.display()
            )
        })?;
        Some((raw, parsed))
    } else {
        None
    };

    let catalog = crawl_catalog(client, config).context("failed to crawl Stark catalog")?;
    let formatted = format_catalog_json5(&catalog).context("failed to format catalog JSON5")?;

    if let Some(parent) = catalog_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create catalog directory {}", parent.display()))?;
    }

    if let Some((existing_raw, existing_catalog)) = existing {
        let mut stable_catalog = catalog.clone();
        stable_catalog.metadata.generated_at = existing_catalog.metadata.generated_at;
        let stable_formatted = format_catalog_json5(&stable_catalog)
            .context("failed to format catalog JSON5 with existing timestamp")?;

        if existing_raw == stable_formatted {
            info!(
                path = %catalog_path.display(),
                "catalog data unchanged; refreshing generated_at"
            );
        }
    }

    fs::write(&catalog_path, formatted)
        .with_context(|| format!("failed to write {}", catalog_path.display()))?;
    info!(path = %catalog_path.display(), "catalog written");
    Ok(format!(
        "catalog written: {}",
        display_path(repo_root, &catalog_path)
    ))
}

fn ensure_repository_root(path: &Path) -> Result<()> {
    let cargo_toml = path.join("Cargo.toml");
    let plan = path.join("PLAN.md");
    let spec = path.join("SPEC.md");

    if cargo_toml.is_file() && plan.is_file() && spec.is_file() {
        Ok(())
    } else {
        bail!(
            "catalog commands must be run from the repository root; {} does not look like it",
            path.display()
        )
    }
}

fn reject_catalog_path_symlinks(repo_root: &Path, path: &Path) -> Result<()> {
    let relative_path = path.strip_prefix(repo_root).with_context(|| {
        format!(
            "catalog path is not under repository root: {}",
            path.display()
        )
    })?;
    let mut current = repo_root.to_path_buf();

    for component in relative_path.components() {
        current.push(component);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                bail!(
                    "catalog path must not contain symlinks: {}",
                    current.display()
                );
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("failed to inspect {}", current.display()));
            }
        }
    }

    Ok(())
}

fn current_rfc3339_timestamp() -> Result<String> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .context("failed to format current timestamp")
}

fn display_path(repo_root: &Path, path: &Path) -> String {
    path.strip_prefix(repo_root)
        .unwrap_or(path)
        .display()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use stark_parts_catalog::{
        UpstreamArticle, UpstreamArticleEntry, UpstreamArticleVariant, UpstreamBikeVariant,
        UpstreamCategory, UpstreamProductDetail, UpstreamProductSummary, UpstreamResult,
    };
    use tempfile::TempDir;

    #[test]
    fn clap_definition_is_valid() {
        use clap::CommandFactory;

        Cli::command().debug_assert();
    }

    #[test]
    fn logging_setup_is_idempotent() {
        init_logging();
        init_logging();
    }

    #[test]
    fn logging_subscriber_can_be_built_with_filter() {
        let subscriber =
            build_logging_subscriber(EnvFilter::try_new("stark_parts_cli=debug").unwrap());

        assert_subscriber(&subscriber);
    }

    #[test]
    fn catalog_init_writes_deterministic_json5_from_repository_root() {
        let repo = repo_root();
        let client = CliFixtureClient::default();
        let cli = Cli::try_parse_from(["stark-parts", "catalog", "init"]).unwrap();
        let config = test_config("2026-05-26T12:34:56Z");

        let message = run_with(cli, repo.path(), &client, config).unwrap();
        let catalog_path = repo.path().join(DEFAULT_CATALOG_PATH);
        let first = fs::read_to_string(&catalog_path).unwrap();

        assert_eq!(message, "catalog written: catalog/stark-parts.json5");
        assert!(first.contains("generated_at: \"2026-05-26T12:34:56Z\""));
        assert!(parse_catalog_json5(&first).is_ok());
    }

    #[test]
    fn catalog_update_refreshes_generated_at_even_when_catalog_data_is_unchanged() {
        let repo = repo_root();
        let client = CliFixtureClient::default();
        let init = Cli::try_parse_from(["stark-parts", "catalog", "init"]).unwrap();
        run_with(
            init,
            repo.path(),
            &client,
            test_config("2026-05-26T12:34:56Z"),
        )
        .unwrap();

        let update = Cli::try_parse_from(["stark-parts", "catalog", "update"]).unwrap();
        let message = run_with(
            update,
            repo.path(),
            &client,
            test_config("2026-05-27T12:34:56Z"),
        )
        .unwrap();

        assert_eq!(message, "catalog written: catalog/stark-parts.json5");
        let catalog = fs::read_to_string(repo.path().join(DEFAULT_CATALOG_PATH)).unwrap();
        assert!(catalog.contains("generated_at: \"2026-05-27T12:34:56Z\""));
    }

    #[test]
    fn catalog_update_writes_new_timestamp_when_catalog_data_changes() {
        let repo = repo_root();
        let client = CliFixtureClient::default();
        let init = Cli::try_parse_from(["stark-parts", "catalog", "init"]).unwrap();
        run_with(
            init,
            repo.path(),
            &client,
            test_config("2026-05-26T12:34:56Z"),
        )
        .unwrap();

        let changed_client = CliFixtureClient {
            article_display_name: "Tall Seat".to_owned(),
        };
        let update = Cli::try_parse_from(["stark-parts", "catalog", "update"]).unwrap();
        let message = run_with(
            update,
            repo.path(),
            &changed_client,
            test_config("2026-05-27T12:34:56Z"),
        )
        .unwrap();
        let catalog = fs::read_to_string(repo.path().join(DEFAULT_CATALOG_PATH)).unwrap();

        assert_eq!(message, "catalog written: catalog/stark-parts.json5");
        assert!(catalog.contains("generated_at: \"2026-05-27T12:34:56Z\""));
        assert!(catalog.contains("display_name: \"Tall Seat\""));
    }

    #[test]
    fn catalog_init_refuses_to_overwrite_existing_catalog() {
        let repo = repo_root();
        let client = CliFixtureClient::default();
        let init = Cli::try_parse_from(["stark-parts", "catalog", "init"]).unwrap();
        run_with(
            init,
            repo.path(),
            &client,
            test_config("2026-05-26T12:34:56Z"),
        )
        .unwrap();

        let init_again = Cli::try_parse_from(["stark-parts", "catalog", "init"]).unwrap();
        let error = run_with(
            init_again,
            repo.path(),
            &client,
            test_config("2026-05-26T12:34:56Z"),
        )
        .unwrap_err();

        assert!(error.to_string().contains("catalog already exists"));
    }

    #[test]
    fn catalog_update_requires_existing_catalog() {
        let repo = repo_root();
        let client = CliFixtureClient::default();
        let update = Cli::try_parse_from(["stark-parts", "catalog", "update"]).unwrap();
        let error = run_with(
            update,
            repo.path(),
            &client,
            test_config("2026-05-26T12:34:56Z"),
        )
        .unwrap_err();

        assert!(error.to_string().contains("catalog does not exist"));
    }

    #[test]
    fn catalog_update_rejects_invalid_existing_catalog_before_writing() {
        let repo = repo_root();
        let client = CliFixtureClient::default();
        let catalog_path = repo.path().join(DEFAULT_CATALOG_PATH);
        fs::create_dir_all(catalog_path.parent().unwrap()).unwrap();
        fs::write(&catalog_path, "{not catalog").unwrap();

        let update = Cli::try_parse_from(["stark-parts", "catalog", "update"]).unwrap();
        let error = run_with(
            update,
            repo.path(),
            &client,
            test_config("2026-05-26T12:34:56Z"),
        )
        .unwrap_err();

        assert!(error.to_string().contains("existing catalog"));
        assert_eq!(fs::read_to_string(catalog_path).unwrap(), "{not catalog");
    }

    #[test]
    fn preflight_rejects_invalid_existing_catalog_before_network_setup() {
        let repo = repo_root();
        let catalog_path = repo.path().join(DEFAULT_CATALOG_PATH);
        fs::create_dir_all(catalog_path.parent().unwrap()).unwrap();
        fs::write(&catalog_path, "{not catalog").unwrap();

        let update = Cli::try_parse_from(["stark-parts", "catalog", "update"]).unwrap();
        let error = preflight(&update, repo.path()).unwrap_err();

        assert!(error.to_string().contains("existing catalog"));
    }

    #[test]
    fn catalog_commands_must_run_from_repository_root() {
        let temp = TempDir::new().unwrap();
        let client = CliFixtureClient::default();
        let init = Cli::try_parse_from(["stark-parts", "catalog", "init"]).unwrap();
        let error = run_with(
            init,
            temp.path(),
            &client,
            test_config("2026-05-26T12:34:56Z"),
        )
        .unwrap_err();

        assert!(
            error
                .to_string()
                .contains("must be run from the repository root")
        );
    }

    #[test]
    fn catalog_update_must_run_from_repository_root() {
        let temp = TempDir::new().unwrap();
        let client = CliFixtureClient::default();
        let update = Cli::try_parse_from(["stark-parts", "catalog", "update"]).unwrap();
        let error = run_with(
            update,
            temp.path(),
            &client,
            test_config("2026-05-26T12:34:56Z"),
        )
        .unwrap_err();

        assert!(
            error
                .to_string()
                .contains("must be run from the repository root")
        );
    }

    #[cfg(unix)]
    #[test]
    fn catalog_update_rejects_catalog_symlinks() {
        use std::os::unix::fs::symlink;

        let repo = repo_root();
        let client = CliFixtureClient::default();
        let catalog_path = repo.path().join(DEFAULT_CATALOG_PATH);
        fs::create_dir_all(catalog_path.parent().unwrap()).unwrap();
        fs::write(repo.path().join("outside.json5"), "{}").unwrap();
        symlink(repo.path().join("outside.json5"), &catalog_path).unwrap();

        let update = Cli::try_parse_from(["stark-parts", "catalog", "update"]).unwrap();
        let error = run_with(
            update,
            repo.path(),
            &client,
            test_config("2026-05-26T12:34:56Z"),
        )
        .unwrap_err();

        assert!(error.to_string().contains("must not contain symlinks"));
    }

    #[cfg(unix)]
    #[test]
    fn catalog_init_rejects_catalog_directory_symlink() {
        use std::os::unix::fs::symlink;

        let repo = repo_root();
        let client = CliFixtureClient::default();
        fs::create_dir(repo.path().join("outside")).unwrap();
        symlink(repo.path().join("outside"), repo.path().join("catalog")).unwrap();

        let init = Cli::try_parse_from(["stark-parts", "catalog", "init"]).unwrap();
        let error = run_with(
            init,
            repo.path(),
            &client,
            test_config("2026-05-26T12:34:56Z"),
        )
        .unwrap_err();

        assert!(error.to_string().contains("must not contain symlinks"));
    }

    struct CliFixtureClient {
        article_display_name: String,
    }

    impl Default for CliFixtureClient {
        fn default() -> Self {
            Self {
                article_display_name: "Original Seat".to_owned(),
            }
        }
    }

    impl UpstreamCatalog for CliFixtureClient {
        fn bike_variants(&self) -> UpstreamResult<Vec<UpstreamBikeVariant>> {
            Ok(vec![UpstreamBikeVariant {
                tag: "varg-ex".to_owned(),
                display_name: Some("Varg EX".to_owned()),
            }])
        }

        fn categories(&self, _tag: &str, _path: &str) -> UpstreamResult<Vec<UpstreamCategory>> {
            Ok(vec![UpstreamCategory {
                code: "bodywork".to_owned(),
                name_key: Some("spare_parts_category_bodywork_name".to_owned()),
                display_name: Some("Bodywork".to_owned()),
                image_url: None,
                is_leaf: true,
                path: "SP".to_owned(),
            }])
        }

        fn products(
            &self,
            _tag: &str,
            _category_code: &str,
        ) -> UpstreamResult<Vec<UpstreamProductSummary>> {
            Ok(vec![UpstreamProductSummary {
                code: "9_seat".to_owned(),
                name_key: Some("spare_parts_product_9_seat_name".to_owned()),
                description_key: Some("spare_parts_product_9_seat_description".to_owned()),
                display_name: Some("Seat".to_owned()),
                description: Some("Seat group".to_owned()),
                image_url: Some(
                    "https://s3-stark-prod.s3.eu-central-1.amazonaws.com/spare-parts-images/Seat.webp"
                        .to_owned(),
                ),
            }])
        }

        fn product_detail(
            &self,
            _tag: &str,
            _country: &str,
            _product_code: &str,
        ) -> UpstreamResult<UpstreamProductDetail> {
            Ok(UpstreamProductDetail {
                code: "9_seat".to_owned(),
                name_key: Some("spare_parts_product_9_seat_name".to_owned()),
                description_key: Some("spare_parts_product_9_seat_description".to_owned()),
                display_name: Some("Seat".to_owned()),
                description: Some("Seat group".to_owned()),
                feature_image_url: None,
                articles: vec![UpstreamArticleEntry {
                    reference: Some(2),
                    article: UpstreamArticle {
                        code: "28_seat_assembly".to_owned(),
                        name_key: Some("spare_parts_product_28_seat_assembly_name".to_owned()),
                        description_key: Some(
                            "spare_parts_product_28_seat_assembly_description".to_owned(),
                        ),
                        display_name: Some(self.article_display_name.clone()),
                        description: Some("Replacement seat".to_owned()),
                        image_url: Some(
                            "https://s3-stark-prod.s3.eu-central-1.amazonaws.com/spare-parts-images/SMX1-P-ST.webp"
                                .to_owned(),
                        ),
                        tags: vec!["varg-ex".to_owned()],
                        is_kit: false,
                        kit_contain: Vec::new(),
                        variants: vec![UpstreamArticleVariant {
                            code: "28_seat_assembly-seat_color.jet_black".to_owned(),
                            skus: vec!["SMX1-P-ST-B".to_owned()],
                            availability: Some("AVAILABLE".to_owned()),
                            price: None,
                            attributes: Vec::new(),
                        }],
                    },
                }],
            })
        }
    }

    fn repo_root() -> TempDir {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("Cargo.toml"), "[workspace]\n").unwrap();
        fs::write(temp.path().join("PLAN.md"), "# plan\n").unwrap();
        fs::write(temp.path().join("SPEC.md"), "# spec\n").unwrap();
        temp
    }

    fn assert_subscriber<T: tracing::Subscriber + Send + Sync>(_subscriber: &T) {}

    fn test_config(generated_at: &str) -> CrawlConfig {
        CrawlConfig::us_storefront(generated_at)
    }
}
