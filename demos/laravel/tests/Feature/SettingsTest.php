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

    public function testThePageShowsTheStoredSetting(): void
    {
        Nativella::fake()->respond('settings.get', ['value' => true]);

        $this->get('/settings')
            ->assertOk()
            ->assertSeeInOrder(['role="switch"', 'checked'], false)
            ->assertSee('Save');
    }

    public function testOpeningAtLoginIsEnabled(): void
    {
        $native = Nativella::fake();

        $this->put('/settings', ['open_at_login' => '1'])->assertRedirect('/settings')->assertSessionHas('status', 'Tasks will open at login.');

        $this->assertCount(1, $native->calls('app.enableAutostart'));
        $this->assertSame([['method' => 'settings.set', 'payload' => ['key' => 'open_at_login', 'value' => true]]], $native->calls('settings.set'));
    }

    public function testOpeningAtLoginIsDisabled(): void
    {
        $native = Nativella::fake();

        $this->put('/settings', ['open_at_login' => '0'])->assertRedirect('/settings');

        $this->assertCount(1, $native->calls('app.disableAutostart'));
        $this->assertSame(['key' => 'open_at_login', 'value' => false], $native->calls('settings.set')[0]['payload']);
    }

    public function testTheSettingIsABoolean(): void
    {
        $native = Nativella::fake();

        $this->from('/settings')->put('/settings', ['open_at_login' => 'often'])->assertRedirect('/settings')->assertSessionHasErrors('open_at_login');

        $this->assertSame([], $native->calls());
    }

    public function testPhonesHaveNothingToSet(): void
    {
        $native = Nativella::fake();
        // What the runtime sets on Android.
        $_SERVER['NATIVELLA_PLATFORM'] = 'android';

        $this->get('/settings')->assertOk()->assertSee('There is nothing to set on this device.')->assertDontSee('Open at login');
        $this->put('/settings', ['open_at_login' => '1'])->assertNotFound();

        $this->assertSame([], $native->calls());
    }

    public function testInABrowserTheSettingsAreReadOnly(): void
    {
        $this->get('/settings')
            ->assertOk()
            ->assertSeeInOrder(['role="switch"', 'disabled>'], false)
            ->assertSee('These settings only exist in the desktop application.');
        $this->put('/settings', ['open_at_login' => '1'])->assertNotFound();
    }
}
