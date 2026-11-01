<?php

namespace worktree;

use function Castor\variable;
use function docker\get_port_specs;

function read_worktree_gitdir(?string $rootDir = null): ?string
{
    $rootDir ??= variable('root_dir');
    $gitFile = $rootDir.'/.git';

    if (!is_file($gitFile)) {
        return null;
    }

    $contents = file_get_contents($gitFile);

    if (false === $contents || !str_starts_with($contents, 'gitdir:')) {
        return null;
    }

    return trim(substr($contents, 7));
}

function get_worktree_name(?string $rootDir = null): ?string
{
    $gitdir = read_worktree_gitdir($rootDir);

    if (null === $gitdir) {
        return null;
    }

    return strtolower(basename(rtrim($gitdir, '/'))) ?: null;
}

/**
 * The ports of the router: those of PROJECT_HTTP_PORT... when set, as
 * docker-compose.dev.yml reads them, or the defaults; in a worktree, ports
 * of its own, derived from its name.
 *
 * @return array<string, int>
 */
function get_worktree_ports(?string $worktreeName): array
{
    $specs = get_port_specs();

    if (null === $worktreeName) {
        $ports = [];
        foreach ($specs as $key => $spec) {
            $configured = getenv($spec['env']);
            if (false === $configured || '' === $configured) {
                $ports[$key] = $spec['default'];

                continue;
            }

            $ports[$key] = filter_var($configured, \FILTER_VALIDATE_INT, ['options' => ['min_range' => 1, 'max_range' => 65535]])
                ?: throw new \RuntimeException(\sprintf('%s is "%s", not a port.', $spec['env'], $configured));
        }

        return $ports;
    }

    $hash = crc32($worktreeName) & 0x7FFFFFFF;
    $base = 10000 + ($hash % 50000);

    $i = 0;
    $ports = [];
    foreach ($specs as $key => $spec) {
        $ports[$key] = $base + $i;
        ++$i;
    }

    return $ports;
}
