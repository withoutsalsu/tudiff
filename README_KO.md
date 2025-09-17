# tudiff

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-blue.svg)](https://www.rust-lang.org)

<!-- [![GitHub](https://img.shields.io/github/stars/withoutsalsu/tudiff?style=social)](https://github.com/withoutsalsu/tudiff) -->

Rust로 작성된 TUI 기반 디렉토리 비교 도구 - Beyond Compare 스타일의 파일 비교 도구입니다.

🇺🇸 [English Documentation](README.md)

## 스크린샷

![tudiff 실행 화면](https://raw.githubusercontent.com/withoutsalsu/tudiff/main/assets/screenshot.png)

## 기능

- **듀얼 패널 디렉토리 트리**: 두 디렉토리를 나란히 비교
- **색상 코딩**:
  - 회색: 동일한 파일/폴더
  - 빨간색: 다른 파일
  - 파란색: 한쪽에만 존재하는 파일/폴더
- **폴더 확장/축소**: Enter 키로 폴더 확장/축소
- **동기화된 탐색**: 스크롤과 폴더 확장 상태가 양쪽 패널에서 동기화
- **빠른 파일 비교**: vimdiff를 사용한 파일 내용 비교
- **세 가지 필터 모드**:
  - 모든 파일 (전체 파일 표시)
  - 다른 파일만 (변경된 파일만 표시)
  - 차이점만 (양쪽에 모두 존재하면서 다른 파일만 표시)
- **인터랙티브 툴바**: 마우스로 툴바 버튼 클릭 가능
- **스마트 복사 기능**: 상태 보존 기능이 포함된 파일 복사
  - 복사 후 커서 위치와 폴더 확장 상태 유지
  - 파일 속성(타임스탬프, 권한) 보존
- **터미널 안전**: 적절한 커서 복원과 패닉 안전 정리

## 설치 및 사용법

### 요구사항

- Rust (1.70+)
- vim 또는 nano (파일 비교용)
- 유니코드 지원 터미널 (이모지 아이콘용)

### 빌드

```bash
git clone https://github.com/withoutsalsu/tudiff.git
cd tudiff
cargo build --release
```

### 사용법

```bash
# 개발 버전 실행
cargo run -- <dir1> <dir2>

# 릴리스 버전 실행
./target/release/tudiff <dir1> <dir2>

# 간단한 텍스트 출력 모드 (TUI 대신)
tudiff --simple <dir1> <dir2>
cargo run -- --simple <dir1> <dir2>
```

**예제:**

```bash
# 두 프로젝트 디렉토리 비교
tudiff ./project-v1 ./project-v2

# 백업과 원본 비교
tudiff ~/Documents /backup/Documents

# 스크립트나 파이프용 간단한 텍스트 출력
tudiff --simple ./project-v1 ./project-v2 | grep "\[L\]"
```

## 조작법

### 마우스 조작

- **툴바 클릭**: 툴바 버튼을 클릭하여 기능 활성화
- **필터 모드**: "모든 파일", "다른 파일만", "차이점만"을 클릭하여 필터 모드 전환
- **액션**: "모두 확장", "모두 축소", "새로고침", "패널 교체"를 클릭

### 키보드 탐색

- `Up/Down`: 파일/폴더 탐색
- `Left/Right`: 왼쪽/오른쪽 패널 간 전환
- `Enter`:
  - 폴더의 경우: 확장/축소
  - 파일의 경우: vimdiff로 비교 (양쪽에 모두 존재하는 경우) 또는 vim으로 단일 파일 열기
- `PageUp/PageDown`: 터미널 높이 기반 반페이지 스크롤
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

성능 최적화를 위한 다단계 비교 사용:

1. **파일 크기 비교** (가장 빠름)
2. **수정 시간 비교** (파일 시스템 차이를 처리하기 위해 1초 허용)
3. **빈 파일 처리** (0바이트 파일은 동일한 것으로 간주)
4. **작은 파일** (< 4KB): 전체 내용 비교
5. **중간 크기 파일** (< 1MB): SHA256 해시 비교
6. **큰 파일** (≥ 1MB): 처음 4KB만 비교

**참고**: 이 알고리즘은 대부분의 사용 사례에서 정확성을 유지하면서 대용량 디렉토리에 대해 뛰어난 성능을 제공합니다.

## UI 개선사항

### 인터랙티브 인터페이스

- **클릭 가능한 툴바**: 모든 툴바 버튼이 마우스로 클릭 가능
- **향상된 키보드 단축키**: 빨간색으로 강조된 키와 함께 단축키 표시 (예: (1), (2), (3))
- **파일 정보 표시**: 각 파일/폴더의 오른쪽에 크기와 수정일 표시
- **색상 코딩 인터페이스**: 다른 파일 상태와 UI 요소에 대한 다른 색상

### 기본 정렬

- **폴더 우선 정렬**: 디렉토리가 항상 파일보다 먼저 나타남
- **대소문자 무시 알파벳순**: 대소문자를 무시하고 파일과 폴더를 알파벳순으로 정렬

## 고급 기능

### 스마트 폴더 상태 감지

폴더는 자식 요소의 상태를 상속받습니다:

- **빨간색**: 자식 파일/폴더가 다른 경우
- **파란색**: 폴더가 한쪽에만 존재하는 경우
- **회색**: 모든 자식이 동일한 경우

이 재귀적 상태 감지는 변경사항이 있는 디렉토리를 빠르게 식별하는 데 도움이 됩니다.

### 터미널 상태 관리

- **커서 복원**: 종료 후 적절한 커서 깜빡임 복원
- **패닉 안전 정리**: 프로그램 충돌 시에도 터미널 상태 복원
- **크로스 플랫폼**: Linux, macOS, Windows 터미널에서 작동

### 성능 최적화

- **백그라운드 작업**: 대용량 디렉토리 스캔이 진행률 표시와 함께 백그라운드에서 실행
- **메모리 효율적**: 모든 것을 메모리에 로드하지 않고 디렉토리 내용을 스트리밍
- **지연 로딩**: 트리 노드가 필요에 따라 채워짐
- **동기화된 스크롤링**: 쉬운 비교를 위해 양쪽 패널이 함께 스크롤

### 오류 처리

- **스마트 에디터 폴백**:
  - 파일 비교: vimdiff → vim -d → diff (사용자 대기 포함)
  - 단일 파일 보기: vim → vi → nano → cat (읽기 전용, 사용자 대기 포함)
- **권한 처리**: 권한 거부 파일이 있어도 작업 계속
- **네트워크 파일시스템**: 느리거나 불안정한 네트워크 마운트를 우아하게 처리
- **크로스 플랫폼 호환성**: 고급 에디터가 없는 최소 시스템에서도 작동

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
- `sha2`: 암호화 해시 함수
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
