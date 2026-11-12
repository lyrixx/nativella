<?php

namespace App\Form;

use Symfony\Component\Form\AbstractType;
use Symfony\Component\Form\Extension\Core\Type\TextareaType;
use Symfony\Component\Form\Extension\Core\Type\TextType;
use Symfony\Component\Form\FormBuilderInterface;
use Symfony\Component\OptionsResolver\OptionsResolver;

/**
 * @extends AbstractType<NoteData>
 */
final class NoteType extends AbstractType
{
    public function buildForm(FormBuilderInterface $builder, array $options): void
    {
        $builder
            ->add('title', TextType::class, [
                // No visible label: the placeholder is not a name for screen readers.
                'attr' => ['aria-label' => 'Title', 'autofocus' => true, 'placeholder' => 'Title'],
                // NotBlank reports an empty title, not the type error of null.
                'empty_data' => '',
                'label' => false,
            ])
            ->add('body', TextareaType::class, [
                'attr' => ['aria-label' => 'Note, in Markdown', 'placeholder' => 'Write in Markdown…', 'rows' => 18],
                'empty_data' => '',
                'label' => false,
                'required' => false,
            ]);
    }

    public function configureOptions(OptionsResolver $resolver): void
    {
        $resolver->setDefaults(['data_class' => NoteData::class]);
    }
}
