<?php

namespace Nativella\Tests;

use Nativella\Client\VersionCheck;
use Nativella\Exception\VersionMismatchException;
use PHPUnit\Framework\Attributes\DataProvider;
use PHPUnit\Framework\Attributes\DoesNotPerformAssertions;
use PHPUnit\Framework\TestCase;

final class VersionCheckTest extends TestCase
{
    /**
     * @param list<string> $packageVersions
     */
    #[DataProvider('provideCompatibleVersions')]
    #[DoesNotPerformAssertions]
    public function testVersionsOfTheSameReleaseLineTalkToEachOther(array $packageVersions, string $runtimeVersion): void
    {
        VersionCheck::ensureCompatible($packageVersions, $runtimeVersion);
    }

    /**
     * @return iterable<string, array{list<string>, string}>
     */
    public static function provideCompatibleVersions(): iterable
    {
        yield 'the same version' => [['0.5.0'], '0.5.0'];
        yield 'a tag with its v' => [['v0.5.1'], '0.5.0'];
        yield 'two patches of a minor' => [['0.5.3'], '0.5.1'];
        yield 'the path repository of the demos' => [['0.5.x-dev'], '0.5.0'];
        yield 'the main branch, by its alias' => [['dev-main', '0.5.x-dev'], '0.5.2'];
        yield 'a branch without alias' => [['dev-fix-the-tray'], '0.5.0'];
        yield 'no version at all' => [[], '0.5.0'];
        yield 'two minors after 1.0' => [['1.4.0'], '1.2.0'];
    }

    public function testAnOlderCommandIsToUpgrade(): void
    {
        $this->expectException(VersionMismatchException::class);
        $this->expectExceptionMessage('The nativella command (0.4.2) is older than the nativella/nativella package (0.5.x-dev): upgrade the command to 0.5.');

        VersionCheck::ensureCompatible(['dev-main', '0.5.x-dev'], '0.4.2');
    }

    public function testAnOlderPackageIsToUpgrade(): void
    {
        $this->expectException(VersionMismatchException::class);
        $this->expectExceptionMessage('The nativella/nativella package (v0.5.3) is older than the nativella command (1.0.0): upgrade the package, and the integration that comes with it, to 1.');

        VersionCheck::ensureCompatible(['v0.5.3'], '1.0.0');
    }

    public function testTenIsAfterNine(): void
    {
        $this->expectException(VersionMismatchException::class);
        $this->expectExceptionMessage('The nativella command (0.9.0) is older than');

        VersionCheck::ensureCompatible(['0.10.0'], '0.9.0');
    }
}
