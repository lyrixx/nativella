<?php

/*
 * Prints, as JSON, the licenses the POM of each Java library declares, or
 * its parent POM, read from the cache of Gradle:
 * {"<group>:<artifact>:<version>": ["<name> (<url>)", …]}, an empty list
 * when the cache has no POM for it, or the POM no license.
 *
 *     php mobile/pom-licenses.php <group>:<artifact>:<version>…
 *
 * For the third-party notices of the template APK (.castor/notices.php),
 * which runs it with the PHP of the builder: the static binary of Castor,
 * which the CI runs, has no XML extension.
 */

$cache = (getenv('GRADLE_USER_HOME') ?: getenv('HOME').'/.gradle').'/caches/modules-2/files-2.1';

$licenses = [];
foreach (array_slice($_SERVER['argv'], 1) as $library) {
    [$group, $artifact, $version] = explode(':', $library, 3);
    $licenses[$library] = pom_licenses($cache, $group, $artifact, $version);
}

echo json_encode($licenses, \JSON_THROW_ON_ERROR | \JSON_UNESCAPED_SLASHES), "\n";

/**
 * @return list<string> "<name> (<url>)"
 */
function pom_licenses(string $cache, string $group, string $artifact, string $version): array
{
    $poms = glob("{$cache}/{$group}/{$artifact}/{$version}/*/{$artifact}-{$version}.pom") ?: [];
    if ([] === $poms) {
        return [];
    }

    $document = new DOMDocument();
    if (!@$document->load($poms[0])) {
        throw new RuntimeException(sprintf('%s is not valid XML.', $poms[0]));
    }

    // A POM has <license> elements in <licenses> only
    $licenses = [];
    foreach ($document->getElementsByTagName('license') as $license) {
        $name = trim($license->getElementsByTagName('name')->item(0)->textContent ?? '');
        $url = trim($license->getElementsByTagName('url')->item(0)->textContent ?? '');
        $licenses[] = '' === $url ? $name : "{$name} ({$url})";
    }

    $parent = $document->getElementsByTagName('parent')->item(0);
    if ([] === $licenses && null !== $parent) {
        $coordinate = static fn (string $name): string => trim($parent->getElementsByTagName($name)->item(0)->textContent ?? '');

        return pom_licenses($cache, $coordinate('groupId'), $coordinate('artifactId'), $coordinate('version'));
    }

    return $licenses;
}
