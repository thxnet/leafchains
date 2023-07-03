{ name
, version
, dockerTools
, thxnet-leafchain
, buildEnv
, ...
}:

dockerTools.buildImage {
  inherit name;
  tag = "v${version}";

  copyToRoot = buildEnv {
    name = "image-root";
    paths = [ thxnet-leafchain ];
    pathsToLink = [ "/bin" ];
  };

  config = {
    Entrypoint = [ "${thxnet-leafchain}/bin/thxnet-leafchain" ];
  };
}
