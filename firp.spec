Name:           firp
Version:        0.2.0
Release:        1%{?dist}
Summary:        A modern Fortran interpreter built in Rust

License:        MIT
URL:            https://github.com/FortranGoingOnForty/firp
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  rust >= 1.70
BuildRequires:  cargo

# Disable debug package
%global debug_package %{nil}

%description
FIRP is a modern interpreter for Fortran, built in Rust with JIT compilation
support via Cranelift. It provides an interactive REPL for executing Fortran
code on the fly.

Features:
- Modern Fortran language support
- Interactive REPL with rustyline
- JIT compilation via Cranelift
- Parallel execution with rayon
- Colorized terminal output

%prep
%autosetup

%build
export CARGO_TARGET_DIR=target
cargo build --release

%install
install -Dm755 target/release/%{name} %{buildroot}%{_bindir}/%{name}

%files
%{_bindir}/%{name}
%doc README.md
%license Cargo.toml

%changelog
* Fri Jan 10 2025 mfw <espadonne@outlook.com> - 0.2.0-1
- Add real-time syntax highlighting to REPL
- Improved user experience with colorized Fortran code

* Fri Jan 10 2025 mfw <espadonne@outlook.com> - 0.1.0-1
- Initial RPM release of firp
- Fortran interpreter built in Rust
- Interactive REPL with JIT compilation
- Multi-architecture support (aarch64, x86_64)
