-- project.db — versão 3 (Fase 3): edição não destrutiva de clipes.
-- Todos os atributos dependem só da chave do clipe (FNBC): ficam como colunas de `clip`.
-- O formato da curva é um domínio fechado e pequeno → CHECK em vez de tabela de referência.
-- Os valores padrão reproduzem exatamente o comportamento das versões anteriores.
ALTER TABLE clip ADD COLUMN fade_in_frames INTEGER NOT NULL DEFAULT 0 CHECK (fade_in_frames >= 0);
ALTER TABLE clip ADD COLUMN fade_out_frames INTEGER NOT NULL DEFAULT 0 CHECK (fade_out_frames >= 0);
ALTER TABLE clip ADD COLUMN fade_in_shape TEXT NOT NULL DEFAULT 'linear'
    CHECK (fade_in_shape IN ('linear', 'equalPower'));
ALTER TABLE clip ADD COLUMN fade_out_shape TEXT NOT NULL DEFAULT 'linear'
    CHECK (fade_out_shape IN ('linear', 'equalPower'));
ALTER TABLE clip ADD COLUMN polarity_inverted INTEGER NOT NULL DEFAULT 0
    CHECK (polarity_inverted IN (0, 1));
ALTER TABLE clip ADD COLUMN reversed INTEGER NOT NULL DEFAULT 0 CHECK (reversed IN (0, 1));
