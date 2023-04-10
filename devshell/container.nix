{ name
, version
, dockerTools
, thxnet-parachain-node
, buildEnv
, ...
}:

dockerTools.buildImage {
  inherit name;
  tag = "v${version}";

  copyToRoot = buildEnv {
    name = "image-root";
    paths = [ thxnet-parachain-node ];
    pathsToLink = [ "/bin" ];
  };

  config = {
    Entrypoint = [ "${thxnet-parachain-node}/bin/thxnet-parachain-node" ];
  };
}
