use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum ProxyErr {
    #[snafu(display("Failed to build matcher: {}", source))]
    BuildMatcher { source: Box<dyn std::error::Error> },
}
