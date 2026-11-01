<?php

namespace Tests\Feature;

use Illuminate\Foundation\Testing\RefreshDatabase;
use Nativella\Laravel\Facades\Nativella;
use Tests\TestCase;

final class SettingsTest extends TestCase
{
    use RefreshDatabase;

    protected function tearDown(): void
    {
        unset($_SERVER['NATIVELLA_PLATFORM']);

        parent::tearDown();
    }

    public function testThePageShowsWhetherTheSystemStartsTheApplication(): void
    {
        Nativella::fake()->respond('app.isAutostartEnabled', ['enabled' => true]);

        $this->get('/settings')
            ->assertOk()
            ->assertSeeInOrder(['role="switch"', 'checked'], false)
            ->assertSee('Save');
    }

    public function testOpeningAtLoginIsEnabled(): void
    {
        $native = Nativella::fake();

        $this->put('/settings', ['autostart' => '1'])->assertRedirect('/settings')->assertSessionHas('status', 'Tasks will open at login.');

        $this->assertSame(['app.enableAutostart'], array_column($native->calls(), 'method'));
    }

    public function testOpeningAtLoginIsDisabled(): void
    {
        $native = Nativella::fake();

        $this->put('/settings', ['autostart' => '0'])->assertRedirect('/settings');

        $this->assertSame(['app.disableAutostart'], array_column($native->calls(), 'method'));
    }

    public function testTheSettingIsABoolean(): void
    {
        $native = Nativella::fake();

        $this->from('/settings')->put('/settings', ['autostart' => 'often'])->assertRedirect('/settings')->assertSessionHasErrors('autostart');

        $this->assertSame([], $native->calls());
    }

    public function testPhonesHaveNothingToSet(): void
    {
        $native = Nativella::fake();
        // What the runtime sets on Android.
        $_SERVER['NATIVELLA_PLATFORM'] = 'android';

        $this->get('/settings')->assertOk()->assertSee('There is nothing to set on this device.')->assertDontSee('Open at login');
        $this->put('/settings', ['autostart' => '1'])->assertNotFound();

        $this->assertSame([], $native->calls());
    }

    public function testInABrowserTheSettingsAreReadOnly(): void
    {
        $this->get('/settings')
            ->assertOk()
            ->assertSeeInOrder(['role="switch"', 'disabled>'], false)
            ->assertSee('These settings only exist in the desktop application.');
        $this->put('/settings', ['autostart' => '1'])->assertNotFound();
    }
}
