<?php

namespace Nativella\ChildProcess;

final readonly class RunningProcess
{
    public function __construct(
        public string $alias,
        public int $pid,
        /** @var non-empty-list<string> */
        public array $command,
    ) {
    }

    /**
     * @param array{alias: string, pid: int, command: non-empty-list<string>} $data
     */
    public static function fromArray(array $data): self
    {
        return new self($data['alias'], $data['pid'], $data['command']);
    }
}
