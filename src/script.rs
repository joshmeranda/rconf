use super::configs::ConfigArchive;

/// Generate the content for an installer script to operate on an unpacked rconf tar.
pub fn build_script(cfg: &ConfigArchive) -> String {
    let mut script = String::from("#!/usr/bin/env bash\n");

    if let Some(specifier) = &cfg.path_specifier {
        if specifier.home.is_some() {
            script.push_str(
                "if [ -d home ];then
    find home -maxdepth 1 -exec cp --recursive --target-directory $HOME '{}' +
fi\n",
            )
        }

        if specifier.config.is_some() {
            script.push_str(
                "if [ -d config ];then
    find config -maxdepth 1 -exec cp --recursive --target-directory $HOME/.config '{}' +
fi\n",
            )
        }

        if specifier.absolute.is_some() {
            script.push_str("abs=($(find . -maxdepth 1 -not \\( -regex './install.sh' -or -regex '.' -or -regex './home.*' -or -regex './.rconf' -or -regex './config.*' \\)))
for file in \"${abs[@]}\"; do
    cp --recursiv $file ${file:1}
done")
        }
    }

    if let Some(manager) = &cfg.manager {
        script.push_str(
            format!(
                "{} {} {}",
                manager.name,
                manager.install_args.join(" "),
                manager.packages.join(" ")
            )
            .as_str(),
        )
    }

    script
}
