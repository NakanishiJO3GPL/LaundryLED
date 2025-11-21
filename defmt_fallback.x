
            SECTIONS
            {
                .defmt :
                {
                    KEEP(*(.defmt))
                    KEEP(*(.defmt.*))
                }
            }
        