fn main() {
    slint_build::compile_with_config("../core/ui/console-preview.slint",
        slint_build::CompilerConfiguration::new()
            .embed_resources(slint_build::EmbedResourcesKind::EmbedForSoftwareRenderer)
            .with_style("fluent-dark".into())).unwrap();
}
