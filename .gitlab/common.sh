install_rgbds() {
    version=${1:-"0.8.0"}
    curl -s -L -o rgbds.tar.xz https://github.com/gbdev/rgbds/releases/download/v$version/rgbds-$version-linux-x86_64.tar.xz
    mkdir -p rgbds
    mv rgbds.tar.xz rgbds/
    pushd rgbds > /dev/null
    tar -xf rgbds.tar.xz
    popd > /dev/null
    export PATH="$CI_PROJECT_DIR/rgbds:$PATH"
    rgbasm --version
}

cargo_publish_all() {
    members=$(cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.publish == null) | .manifest_path')
    for member in $members; do
        echo "Publishing crate: $member"
        cargo publish --manifest-path "$member" --no-verify
    done
}
