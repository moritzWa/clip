# Dependency notices

Some published objc2 workspace crates declare an MIT license without including
the workspace-level license text in their crate archive. The release packager
uses objc2-MIT.txt for those crates and copies all other notices directly from
the Cargo-resolved packages.

The attributed MIT notice was retrieved from the upstream repository at commit
b98d2a3e0f7b7adf83cf12c16e834752f16b0b51:
https://github.com/madsmtm/objc2/blob/b98d2a3e0f7b7adf83cf12c16e834752f16b0b51/LICENSE-MIT.txt

The app's Resources/ThirdPartyLicenses directory includes the dependency inventory
and the Rust standard library's component copyright and license notices. These
third-party notices do not replace Clipglass's own MIT license.
