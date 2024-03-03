# Adapted from https://github.com/replit/rippkgs/blob/main/lib/default.nix
{lib, ...}: let
in {
  genRegistry = platform: pkgs: let
    inherit (builtins) deepSeq filter listToAttrs map parseDrvName seq tryEval;
    inherit (lib) filterAttrs flatten foldl isDerivation mapAttrsToList optional optionals removePrefix traceVal;

    registerPackage = name: value: let
      safeValue = tryEval value;
      safeVal = safeValue.value;

      safeRegistryValue = tryEval (deepSeq registryValue registryValue);
      registryValue = {
        version = safeVal.version or null;
        storePaths = let
          getOutput = out: {
            name = out;
            value = let
              outPath = tryEval safeVal.${out}.outPath;
            in if outPath.success then removePrefix "/nix/store/" outPath.value else "<broken>";
          };

          outputs-list = map getOutput (safeVal.outputs or []);
          relevant-outputs = filter ({name, ...}: name == "out") outputs-list;
        in
          listToAttrs relevant-outputs;
      };

      platformForAvailability = {system = platform;};
      isAvailableOn = tryEval (lib.meta.availableOn platformForAvailability safeValue.value);
      available = safeValue.success && isDerivation value && isAvailableOn.success && isAvailableOn.value;

      checkRegistryCondition = prev: {
        reason,
        ok,
      }: let
        isOk =
          if !ok
          then seq (traceVal "${name}: ${reason}") false
          else true;
      in
        # change to `prev && isOk` to debug why a value isn't included
        prev && ok;

      shouldBeInRegistry = foldl checkRegistryCondition true [
        {
          reason = "not available on ${platformForAvailability.system}";
          ok = available;
        }
        {
          reason = "failed eval";
          ok = safeRegistryValue.success;
        }
        {
          reason = "broken outpath";
          ok = safeRegistryValue.value.storePaths != { out = "<broken>"; } && safeRegistryValue.value.storePaths != {};
        }
      ];
    in
      optional shouldBeInRegistry {
        inherit name;
        value = filterAttrs (_: v: v != null) safeRegistryValue.value;
      };

    registerScope = scope-name: scope: let
      safeScope = tryEval scope;

      list-of-scope-packages = mapAttrsToList registerPackage safeScope.value;
      scope-registry-inner = flatten list-of-scope-packages;
      scope-registry =
        map (item: {
          name = "${scope-name}.${item.name}";
          value = item.value;
        })
        scope-registry-inner;

      shouldBeInRegistry = safeScope.success && safeScope.value ? recurseForDerivations && safeScope.value.recurseForDerivations;
    in
      optionals shouldBeInRegistry scope-registry;

    list-of-registry-packages = mapAttrsToList registerPackage pkgs;
    registry-items = flatten list-of-registry-packages;

    scoped-registries = flatten (mapAttrsToList registerScope pkgs);
    registry = listToAttrs (registry-items ++ scoped-registries);
  in
    registry;
}
