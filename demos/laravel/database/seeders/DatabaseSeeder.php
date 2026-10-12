<?php

namespace Database\Seeders;

use App\Models\Task;
use Illuminate\Database\Console\Seeds\WithoutModelEvents;
use Illuminate\Database\Seeder;

final class DatabaseSeeder extends Seeder
{
    use WithoutModelEvents;

    public function run(): void
    {
        Task::factory()->completed()->create(['title' => 'Install Nativella']);
        Task::factory()->create(['title' => 'Water the plants']);
        Task::factory()->create(['title' => 'Call the dentist']);
    }
}
