# tudiff

[![Crates.io](https://img.shields.io/crates/v/tudiff)](https://crates.io/crates/tudiff)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-blue.svg)](https://www.rust-lang.org)
[![GitHub](https://img.shields.io/github/stars/withoutsalsu/tudiff?style=social)](https://github.com/withoutsalsu/tudiff)

Rust로 작성된 고성능 터미널 디렉토리 비교 도구 - Beyond Compare의 직관적인 인터페이스를 터미널 환경으로 가져온 파일 비교 도구입니다.

🇺🇸 [English Documentation](README.md)

## 스크린샷

![tudiff 실행 화면](https://raw.githubusercontent.com/withoutsalsu/tudiff/main/assets/screenshot.png)

## 기능

- **듀얼 패널 디렉토리 트리**: 두 디렉토리를 나란히 배치하여 쉽게 비교
- **스마트 색상 표시**:
  - 회색: 동일한 파일/폴더
  - 빨간색: 내용이 다른 파일
  - 파란색: 한쪽에만 있는 파일/폴더
- **폴더 탐색**: Enter 키로 폴더를 펼치거나 접어서 빠르게 탐색
- **동기화된 스크롤**: 양쪽 패널의 스크롤과 폴더 확장 상태가 자동으로 동기화
- **빠른 파일 비교**: vimdiff로 파일 내용의 차이를 바로 확인
- **3가지 필터 모드**:
  - 전체 보기: 모든 파일과 폴더 표시
  - 차이점 보기: 변경된 항목만 표시
  - 차이만 보기: 양쪽에 있으면서 내용이 다른 파일만 표시
- **마우스 지원 툴바**: 마우스로 클릭해서 기능 사용 가능
- **스마트 파일 복사**: 상태를 유지하며 파일 복사
  - 복사 후 커서 위치와 폴더 확장 상태 유지
  - 파일 속성(날짜, 권한) 보존
- **안전한 터미널 관리**: 비정상 종료 시에도 커서 상태 복원

## 설치 및 사용법

### 요구사항

- Rust (1.70+)
- vim 또는 nano (파일 비교용)
- 유니코드 지원 터미널 (이모지 아이콘용)

### crates.io에서 설치

```bash
cargo install tudiff
```

### 소스에서 빌드

```bash
git clone https://github.com/withoutsalsu/tudiff.git
cd tudiff

# 빌드만 실행
cargo build --release

# 또는 cargo bin 디렉토리에 설치
cargo install --path .
```

### 사용법

```bash
# 개발 버전 실행
cargo run -- <dir1> <dir2>

# 릴리스 버전 실행
./target/release/tudiff <dir1> <dir2>

# cargo install로 설치한 경우
tudiff <dir1> <dir2>

# 간단한 텍스트 출력 모드 사용 (TUI 대신, 스크립팅이나 파이핑에 유용)
tudiff --simple <dir1> <dir2>
cargo run -- --simple <dir1> <dir2>

# 상세 로깅 활성화 (tudiff.log 파일 생성)
tudiff --verbose <dir1> <dir2>
tudiff -v <dir1> <dir2>
cargo run -- --verbose <dir1> <dir2>
```

**예제:**

```bash
# 두 프로젝트 디렉토리 비교
tudiff ./project-v1 ./project-v2

# 백업과 원본 비교
tudiff ~/Documents /backup/Documents

# 스크립트나 파이프용 간단한 텍스트 출력
tudiff --simple ./project-v1 ./project-v2 | grep "\[L\]"

# 디버깅용 상세 로그 활성화
tudiff --verbose ./project-v1 ./project-v2
```

## 조작법

### 마우스 조작

- **툴바 클릭**: 툴바 버튼을 클릭하여 기능 활성화
- **필터 모드**: "모든 파일", "다른 파일만", "차이점만"을 클릭하여 필터 모드 전환
- **액션**: "모두 확장", "모두 축소", "새로고침", "패널 교체"를 클릭
- **마우스 휠**: 위/아래 스크롤로 파일 목록 탐색

### 키보드 탐색

- `Up/Down` 또는 `j/k`: 파일/폴더 탐색
- `Left/Right` 또는 `h/l`: 왼쪽/오른쪽 패널 간 전환
- `Enter`:
  - 폴더의 경우: 확장/축소
  - 파일의 경우: vimdiff로 비교 (양쪽에 모두 존재하는 경우) 또는 vim으로 단일 파일 열기
- `PageUp/PageDown` 또는 `Ctrl+B/Ctrl+F`: 터미널 높이 기반 반페이지 스크롤
- `Ctrl+Home`: 맨 위로 스크롤
- `Ctrl+End`: 맨 아래로 스크롤
- `1`: 모든 파일 표시
- `2`: 다른 파일만 표시
- `3`: 차이점만 표시 (양쪽에 모두 존재하는 파일만)
- `+`: 모든 폴더 확장
- `-`: 모든 폴더 축소
- `F5`: 디렉토리 새로고침
- `s`: 패널 내용 교체
- `Ctrl+R` / `Ctrl+L`: 선택된 파일 복사 (왼쪽→오른쪽 / 오른쪽→왼쪽)
- `q` 또는 `Esc`: 종료

### 화면 레이아웃

```
┌────────── 🛠️  Tools ─────────────────────────────────────────────────────────┐
│ 📁 All Files(1) │ 🔍 Different(2) │ ⚡ Diff Only(3) │ 📂 Expand All(+) │
│ 📁 Collapse All(-) │ 🔄 Refresh(F5) │ 🔃 Swap Panels(s) │ ▶️Copy(Ctrl+R) │
│ Filter: All Files                                                         │
└───────────────────────────────────────────────────────────────────────────────┘
┌─────────── Left: /path/to/dir1 ──────────┐┌─────────── Right: /path/to/dir2 ─────────┐
│ 📁 folder1                   2.5K Mar 15││ 📁 folder1                   2.5K Mar 15│
│   📄 file1.txt               1.2K Mar 10││   📄 file1.txt               1.2K Mar 10│
│   📄 file2.txt               3.4K Mar 12││                                         │
│ 📁 folder2                   5.1K Mar 14││ 📁 folder2                   8.2K Mar 16│
│                                         ││   📁 subfolder               1.8K Mar 16│
│                                         ││     📄 newfile.txt           1.8K Mar 16│
└─────────────────────────────────────────┘└─────────────────────────────────────────┘
```

## 파일 비교 알고리즘

빠르고 정확한 비교를 위한 단계별 처리:

1. **1단계: 파일 크기 비교** (가장 빠름) - 크기가 다르면 즉시 다르다고 판단
2. **2단계: 빈 파일 처리** - 0바이트 파일은 즉시 같다고 판단
3. **3단계: 작은 파일** (< 4KB) - 전체 내용을 직접 비교
4. **4단계: 중간 파일** (< 1MB) - CRC32 해시로 빠르게 비교
5. **5단계: 큰 파일** (≥ 1MB) - 앞부분 4KB만 비교해서 빠르게 처리

**참고**: 이 방식은 대용량 디렉토리에서도 빠른 속도와 정확성을 모두 제공합니다.

## UI 개선사항

### 편리한 사용자 인터페이스

- **마우스 사용 가능**: 모든 툴바 버튼을 마우스로 클릭 가능
- **단축키 표시**: 단축키를 빨간색으로 강조해서 쉽게 확인 (예: (1), (2), (3))
- **파일 정보 표시**: 파일 크기와 수정 날짜를 각 항목 옆에 표시
- **색상으로 구분**: 파일 상태에 따라 다른 색상으로 표시해서 한눈에 파악

### 기본 정렬

- **폴더 우선 정렬**: 디렉토리가 항상 파일보다 먼저 나타남
- **대소문자 무시 알파벳순**: 대소문자를 무시하고 파일과 폴더를 알파벳순으로 정렬

## 고급 기능

### 스마트 폴더 상태 표시

폴더 색상은 하위 항목의 상태를 반영합니다:

- **빨간색**: 하위 항목 중 하나 이상이 변경됨
- **파란색**: 폴더가 한쪽에만 존재
- **회색**: 모든 하위 항목이 동일함

이 방식으로 변경사항이 있는 폴더를 빠르게 찾을 수 있습니다.

### 터미널 상태 관리

- **자동 커서 복원**: 프로그램 종료 시 터미널 커서 자동 복구
- **안전한 종료**: 비정상 종료 시에도 터미널 상태 복원
- **다중 플랫폼 지원**: Linux, macOS, Windows에서 모두 동작

### 성능 최적화

- **백그라운드 스캔**: 대용량 디렉토리 분석 시 진행률을 보여주며 백그라운드에서 처리
- **메모리 효율**: 디렉토리 내용을 스트리밍 방식으로 처리해서 메모리 절약
- **필요 시 로딩**: 필요할 때만 트리 노드를 만들어서 초기 로딩 시간 단축
- **스크롤 동기화**: 양쪽 패널의 스크롤을 자동으로 맞춰서 비교 편의성 향상

### 오류 처리

- **스마트 에디터 선택**:
  - 파일 비교: vimdiff → vim -d → diff (자동으로 사용 가능한 에디터 선택)
  - 단일 파일 보기: vim → vi → nano → cat (시스템에 맞춰 자동 선택)
- **권한 오류 우회**: 접근 불가능한 파일이 있어도 계속 스캔
- **네트워크 파일시스템**: 느리거나 불안정한 네트워크 드라이브도 안정적으로 처리
- **최소 요구사항**: 기본 시스템 도구만 있어도 모든 기능 사용 가능

## 사용 사례

### 개발 워크플로우

```bash
# 브랜치 비교
tudiff ./main-branch ./feature-branch

# 리팩토링 전/후 비교
tudiff ./before-refactor ./after-refactor

# 배포된 것 vs 로컬 비교
tudiff ./local-project /mnt/server/deployed-project
```

### 시스템 관리

```bash
# 구성 드리프트 감지
tudiff /etc/nginx /backup/etc/nginx

# 백업 검증
tudiff /home/user /backup/home/user

# 시스템 상태 비교
tudiff /var/log /backup/var/log
```

### 데이터 마이그레이션

```bash
# 파일 전송 검증
tudiff /source/data /destination/data

# 디렉토리 구조 비교
tudiff /old-system/files /new-system/files
```

## 문제 해결

### 일반적인 문제

**종료 후 터미널 커서가 깜빡이지 않음:**

- tudiff의 터미널 복원에 의해 자동으로 처리됨
- 문제가 지속되면 실행: `tput cnorm`

**유니코드 아이콘이 표시되지 않음:**

- 터미널이 유니코드/UTF-8을 지원하는지 확인
- 대부분의 최신 터미널에서 기본적으로 지원됨

**대용량 디렉토리에서의 성능:**

- "다른 파일만" 필터 (키 `2`)를 사용하여 표시 항목 줄이기
- 10만개 이상의 파일이 있는 디렉토리에 최적화되어 있음

**권한 오류:**

- 도구는 계속 스캔하고 접근할 수 없는 파일을 적절하게 표시함
- 전체 액세스를 위해 적절한 권한으로 실행

## 의존성

주요 Rust 크레이트:

- `ratatui`: TUI 인터페이스 프레임워크
- `crossterm`: 크로스 플랫폼 터미널 조작
- `clap`: 명령행 인수 파싱
- `walkdir`: 효율적인 디렉토리 순회
- `similar`: 텍스트 차이 알고리즘
- `crc32fast`: 빠른 CRC32 체크섬 계산
- `anyhow`: 오류 처리 및 컨텍스트

## 라이선스

MIT License

## 유사한 도구

**tudiff** vs 다른 디렉토리 비교 도구:

- **Beyond Compare**: 상용, GUI 기반, 더 많은 기능이지만 터미널 네이티브가 아님
- **diff -r**: 명령행, 텍스트 출력만, 인터랙티브 탐색 없음
- **meld**: GUI 기반, X11/데스크톱 환경 필요
- **kdiff3**: GUI 기반, 무거운 의존성 요구사항

**tudiff**는 두 장점을 모두 제공합니다: 가벼운, 터미널 네이티브 인터페이스에서 강력한 비교 기능.
