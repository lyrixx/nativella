<?php

namespace Nativella\Tests;

use Nativella\Runtime;
use PHPUnit\Framework\Attributes\BackupGlobals;
use PHPUnit\Framework\TestCase;

#[BackupGlobals(true)]
final class RuntimeTest extends TestCase
{
    public function testItReadsWhatTheRuntimeSets(): void
    {
        $_SERVER['NATIVELLA_DATA_DIR'] = '/home/me/.local/share/dev.nativella.notes';
        $_SERVER['NATIVELLA_PLATFORM'] = 'android';
        $_SERVER['NATIVELLA_WINDOW'] = 'main';

        $this->assertSame('/home/me/.local/share/dev.nativella.notes', Runtime::dataDir());
        $this->assertSame('android', Runtime::platform());
        $this->assertTrue(Runtime::isMobile());
        $this->assertSame('main', Runtime::window());
        $this->assertFalse(Runtime::isInternalRequest());
    }

    public function testOutsideNativellaThereIsNothing(): void
    {
        unset($_SERVER['NATIVELLA_DATA_DIR'], $_SERVER['NATIVELLA_PLATFORM']);

        $this->assertFalse(Runtime::isNative());
        $this->assertNull(Runtime::dataDir());
        $this->assertFalse(Runtime::isMobile());
    }
}
